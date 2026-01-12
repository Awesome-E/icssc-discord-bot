use std::{collections::HashSet, str::FromStr as _};

use anyhow::{Context as _, Error, Result, bail};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use itertools::Itertools as _;
use serenity::{
    all::{
        CacheHttp as _, CreateActionRow, CreateInputText, CreateInteractionResponse, CreateModal,
        EditInteractionResponse, InputTextStyle, ModalInteraction, ReactionType, UserId,
    },
    futures::future,
};

use crate::{
    AppError, AppVars, Context,
    util::{
        ContextExtras as _,
        gsheets::{TokenResponse, get_gsheets_token, get_spreadsheet_range},
        message::get_members,
        modal::ModalInputTexts,
        roster::{check_in_with_email, get_bulk_members_from_roster, get_user_from_discord},
    },
};

/// Check into today's ICSSC event!
#[poise::command(slash_command, hide_in_help)]
pub(crate) async fn checkin(ctx: Context<'_>) -> Result<(), Error> {
    let Ok(TokenResponse { access_token }) = get_gsheets_token(ctx.data()).await else {
        ctx.reply_ephemeral("Unable to find who you are :(").await?;
        return Ok(());
    };

    ctx.defer_ephemeral().await?;

    let username = &ctx.author().name;
    let Ok(Some(user)) =
        get_user_from_discord(ctx.data(), Some(&access_token), username.to_string()).await
    else {
        ctx.reply_ephemeral(
            "\
Cannot find a matching internal member. Double check that your \
Discord username on the internal roster is correct.",
        )
        .await?;
        return Ok(());
    };

    let success = check_in_with_email(ctx.data(), &user.email, None)
        .await
        .is_ok();
    if !success {
        ctx.reply_ephemeral("Unable to check in").await?;
        return Ok(());
    };

    ctx.reply_ephemeral(format!("Successfully checked in as {}", user.name))
        .await?;
    Ok(())
}

/// Count a message as attendance for an ICSSC event
#[poise::command(context_menu_command = "Log Attendance", guild_only)]
pub(crate) async fn log_attendance(
    ctx: Context<'_>,
    message: serenity::all::Message,
) -> Result<(), Error> {
    let Context::Application(ctx) = ctx else {
        bail!("unexpected context type")
    };

    let members: HashSet<String> = get_members(&message, true);

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
    ctx: &serenity::prelude::Context,
    data: &'_ AppVars,
    ixn: &ModalInteraction,
) -> Result<(), AppError> {
    let inputs = ModalInputTexts::new(ixn);
    let message = inputs
        .get_required_value("message_id")?
        .parse::<u64>()
        .context("unexpected non-numerical message ID")
        .map(|id| ixn.channel_id.message(ctx.http(), id))?
        .await?;

    let attendees = inputs.get_required_value("participants")?;
    let event_name = inputs.get_value("event_name")?;

    let participant_ids = attendees.split("\n");
    let participants = future::join_all(participant_ids.clone().filter_map(|s| {
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

    let usernames = participants
        .iter()
        .map(|member| member.user.name.clone())
        .collect_vec();

    let members = get_bulk_members_from_roster(data, &usernames).await?;
    let is_missing = members.len() != usernames.len();
    if is_missing {
        bail!("user lookup failed");
    };

    ixn.defer_ephemeral(ctx.http()).await?;

    let event_name = event_name.as_deref();
    let mut response_lines = Vec::new();
    for member in members {
        let success = check_in_with_email(data, &member.email, event_name)
            .await
            .is_ok();
        let emoji = match success {
            true => "☑️",
            false => "❌",
        };
        let line = format!("{} {} ({})", emoji, member.name, member.email);
        response_lines.push(line);
    }

    let content = String::from("Submitted attendance for the following users:\n")
        + &response_lines.join("\n");

    ixn.edit_response(ctx.http(), EditInteractionResponse::new().content(content))
        .await?;

    let _ = message
        .react(ctx.http(), ReactionType::Unicode("👋".to_string()))
        .await;

    Ok(())
}

async fn get_events_attended_text(
    data: &AppVars,
    access_token: Option<&str>,
    email: &String,
) -> Result<Vec<String>, AppError> {
    let sheet_id = &data.env.attendance_sheet.id;
    let range = &data.env.attendance_sheet.ranges.checkin;
    let resp = get_spreadsheet_range(data, sheet_id, range, access_token).await?;

    let events = resp
        .values
        .into_iter()
        .filter_map(|row| {
            let row = row.into_iter().collect_array::<4>()?;
            let [time, row_email, _, name] = row;

            if row_email != *email {
                return None;
            };

            let current_time = Utc::now().time();
            let datetime = NaiveDateTime::parse_from_str(&time, "%m/%d/%Y %H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(&time, "%m/%d/%y %H:%M:%S"))
                .or_else(|_| {
                    NaiveDate::parse_from_str(&time, "%m/%d/%y")
                        .map(|res| res.and_time(current_time))
                })
                .or_else(|_| {
                    NaiveDate::parse_from_str(&time, "%m/%d/%Y")
                        .map(|res| res.and_time(current_time))
                })
                .ok()?;

            Some(format!("- <t:{}:d> {name}", datetime.and_utc().timestamp()))
        })
        .collect_vec();

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
    let Ok(Some(user)) =
        get_user_from_discord(ctx.data(), Some(&access_token), username.to_string()).await
    else {
        ctx.reply_ephemeral(
            "\
Cannot find a matching internal member. Double check that your \
Discord username on the internal roster is correct.",
        )
        .await?;
        return Ok(());
    };

    let events = get_events_attended_text(ctx.data(), Some(&access_token), &user.email).await?;

    ctx.reply_ephemeral(format!("Events you attended:\n{}", events.join("\n")))
        .await?;
    Ok(())
}
