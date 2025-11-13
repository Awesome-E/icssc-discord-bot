use std::{collections::HashSet, str::FromStr};

use anyhow::{Context as _, Error, Result, bail};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use itertools::Itertools;
use serde::{Deserialize};
use serenity::{all::{CacheHttp, CreateActionRow, CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, InputTextStyle, MessageId, ModalInteraction, ReactionType, UserId}, futures::future};

use crate::{AppError, AppVars, Context, attendance::roster_helpers::{TokenResponse, check_in_with_email, get_gsheets_token, get_user_from_discord}, util::ContextExtras};

#[derive(Debug, Deserialize)]
struct FlexibleSheetsResp {
    values: Vec<Vec<String>>,
}

/// Check into today's ICSSC event!
#[poise::command(slash_command, hide_in_help)]
pub(crate) async fn checkin(ctx: Context<'_>) -> Result<(), Error> {
    let Ok(TokenResponse { access_token }) = get_gsheets_token(ctx.data()).await else {
        ctx.reply_ephemeral("Unable to find who you are :(").await?;
        return Ok(());
    };

    ctx.defer_ephemeral().await?;

    let username = &ctx.author().name;
    let Ok(Some(user)) = get_user_from_discord(ctx.data(), &access_token, username.to_string()).await else {
        ctx.reply_ephemeral(
            "\
Cannot find a matching internal member. Double check that your \
Discord username on the internal roster is correct.",
        )
        .await?;
        return Ok(());
    };

    let success = check_in_with_email(ctx.data(), user.email, None).await.is_ok();
    if !success {
        ctx.reply_ephemeral("Unable to check in").await?;
        return Ok(());
    };

    ctx.reply_ephemeral(format!("Successfully checked in as {}", user.name))
        .await?;
    Ok(())
}

/// Count a message as attendance for an ICSSC event
#[poise::command(context_menu_command = "Log Attendance")]
pub(crate) async fn log_attedance (ctx: Context<'_>, message: serenity::all::Message) -> Result<(), Error> {
    let Context::Application(ctx) = ctx else {
        bail!("unexpected context type")
    };

    let mut members: HashSet<String> = message
        .mentions
        .iter().map(|member| member.id.to_string())
        .collect();
    members.insert(message.author.id.to_string());

    // create inputs
    let msg_input: CreateActionRow = CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Message ID", "message_id")
            .value(message.id.to_string())
            .required(true),
    );
    let event_name_input = CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Name of Event", "event_name")
            .value("")
            .required(false),
    );
    let members_input = CreateActionRow::InputText(
        CreateInputText::new(
            InputTextStyle::Paragraph,
            "Who was at this event?",
            "participants",
        )
        .value(members.iter().join("\n"))
        .required(true),
    );

    let modal = CreateModal::new("attendance_log_modal_confirm", "Confirm Attendance")
        .components(vec![msg_input, event_name_input, members_input]);

    let reply = CreateInteractionResponse::Modal(modal);
    ctx.interaction.create_response(ctx.http(), reply).await?;

    Ok(())
}

pub(crate) async fn confirm_attendance_log_modal(
    ctx: serenity::prelude::Context,
    data: &'_ AppVars,
    ixn: ModalInteraction,
) -> Result<(), AppError> {
    let inputs = ixn
        .data
        .components
        .iter()
        .filter_map(|row| {
            let item = row.components[0].clone();
            match item {
                serenity::all::ActionRowComponent::InputText(item) => Some(item),
                _ => None,
            }
        })
        .collect_vec();

    let Some(message_id) = inputs
        .iter()
        .find(|input| input.custom_id == "message_id")
    else {
        bail!("unexpected missing input")
    };

    let Ok(message_id) = message_id.value.clone().map_or(Ok(0 as u64), |s| s.parse()) else {
        bail!("unexpected non-numerical message ID")
    };

    let message = ixn
        .channel_id
        .message(ctx.http(), MessageId::new(message_id))
        .await?;

    let Some(attendees) = inputs
        .iter()
        .find(|input| input.custom_id == "participants")
    else {
        bail!("unexpected missing input")
    };
    let Some(attendees_value) = &attendees.value else { bail!("unexpected empty input") };

    let Some(event_name) = inputs
        .iter()
        .find(|input| input.custom_id == "event_name")
        .map(|input| &input.value)
    else {
        bail!("unexpected missing input")
    };

    let participant_ids = attendees_value.split("\n");
    let participants = future::join_all(
        participant_ids.clone()
        .filter_map(|s| {
            let uid = UserId::from_str(s.trim()).ok()?;
            Some(ixn.guild_id?.member(ctx.http(), uid))
        }))
        .await
        .into_iter()
        .filter_map(|item| item.ok())
        .collect_vec();

    if participant_ids.collect_vec().len() != participants.len() {
        bail!("Some user IDs not found");
    }

    let access_token = get_gsheets_token(data).await?.access_token;
    let mut emails: Vec<String> = Vec::new();
    let mut user_texts: Vec<String> = Vec::new();
    for member in participants {
        let user = get_user_from_discord(data, &access_token, member.user.name)
            .await?
            .context(format!("cannot find email for <@{}>", member.user.id.to_string()))?;
        emails.push(user.email.clone());
        user_texts.push(format!("- {} ({})", user.name, user.email));
    }

    for email in emails { check_in_with_email(data, email, event_name.clone()).await?; };

    let content = String::from("Successfully logged attendance for the following users:\n") +
        &user_texts.join("\n");

    ixn.create_response(
        ctx.http(),
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true),
        ),
    )
    .await?;

    let _ = message
        .react(ctx.http(), ReactionType::Unicode("👋".to_string()))
        .await;

    Ok(())
}

async fn get_events_attended_text(data: &AppVars, access_token: &String, email: &String) -> Result<Vec<String>, AppError> {
    let spreadsheet_id = &data.env.attendance_sheet.id;
    let spreadsheet_range = &data.env.attendance_sheet.ranges.checkin;

    let resp = reqwest::Client::new()
        .get(format!("https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{spreadsheet_range}"))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<FlexibleSheetsResp>()
        .await?;

    let events = resp.values.into_iter().filter_map(|row| {
        if row.len() != 4 { return None; };
        let Some(row ) = row.into_iter().collect_array() else {
            return None;
        };
        let [time, row_email, _, name] = row;

        if row_email != *email { return None; };

        let noon = NaiveTime::parse_from_str("20:00:00", "%H:%M:%S").expect("parse noon");
        let mut datetime = NaiveDateTime::parse_from_str(&time, "%m/%d/%Y %H:%M:%S");
        if let Err(_) = datetime { datetime = NaiveDateTime::parse_from_str(&time, "%m/%d/%y %H:%M:%S"); };
        if let Err(_) = datetime {
            datetime = NaiveDate::parse_from_str(&time, "%m/%d/%y").and_then(|res| Ok(res.and_time(noon)))
        };
        if let Err(_) = datetime {
            datetime = NaiveDate::parse_from_str(&time, "%m/%d/%Y").and_then(|res| Ok(res.and_time(noon)))
        };
        let Ok(datetime) = datetime else { return None; };

        Some(format!("- <t:{}:d> {name}", datetime.and_utc().timestamp()))
    }).collect_vec();

    Ok(events)
}

/// See what ICSSC events you have checked in for!
#[poise::command(slash_command, hide_in_help)]
pub(crate) async fn attended(ctx: Context<'_>) -> Result<(), Error> {
    let Ok(TokenResponse { access_token }) = get_gsheets_token(ctx.data()).await else {
        ctx.reply_ephemeral("Unable to find who you are :(").await?;
        return Ok(());
    };

    ctx.defer_ephemeral().await?;

    let username = &ctx.author().name;
    let Ok(Some(user)) = get_user_from_discord(ctx.data(), &access_token, username.to_string()).await else {
        ctx.reply_ephemeral(
            "\
Cannot find a matching internal member. Double check that your \
Discord username on the internal roster is correct.",
        )
        .await?;
        return Ok(());
    };

    let events = get_events_attended_text(ctx.data(), &access_token, &user.email).await?;

    ctx.reply_ephemeral(format!("Events you attended:\n{}", events.join("\n"))).await?;
    Ok(())
}
