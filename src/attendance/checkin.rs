use anyhow::{Error, Result, bail};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use itertools::Itertools;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{AppError, AppVars, Context, util::ContextExtras};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    scope: String,
    aud: String,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    access_token: String,
    // Can also read token_type: String and expires_in: u64
}

#[derive(Debug, Deserialize)]
struct SheetsRow {
    name: String,
    email: String,
    discord: String,
}

#[derive(Debug, Deserialize)]
struct RosterSheetsResp {
    values: Vec<[String; 3]>,
}

#[derive(Debug, Deserialize)]
struct FlexibleSheetsResp {
    values: Vec<Vec<String>>,
}


async fn get_gsheets_token(data: &AppVars) -> Result<TokenResponse, AppError> {
    let key = &data.env.service_account_key;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let now = now.as_secs();

    let claims = Claims {
        iss: key.email.clone(),
        sub: key.email.clone(),
        scope: "https://www.googleapis.com/auth/spreadsheets.readonly".to_owned(),
        aud: "https://oauth2.googleapis.com/token".to_owned(),
        exp: now + 3600,
        iat: now,
    };

    let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(key.id.to_owned());

    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(key.pem.as_bytes())?,
    )
    .unwrap();

    let token_resp = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &token),
        ])
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;

    Ok(token_resp)
}

async fn get_user_from_discord(
    data: &AppVars,
    access_token: &String,
    username: String,
) -> Result<Option<SheetsRow>, AppError> {
    let spreadsheet = &data.env.roster_spreadsheet;

    let resp = reqwest::Client::new()
        .get(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet.id,
            spreadsheet.range
        ))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<RosterSheetsResp>()
        .await?;

    let user = resp
        .values
        .iter()
        .map(|row| {
            let [name, email, discord] = row;
            SheetsRow {
                name: name.to_string(),
                email: email.to_string(),
                discord: discord.to_string(),
            }
        })
        .find(|row| row.discord.to_lowercase() == username);

    Ok(user)
}

async fn check_in_with_email(data: &AppVars, email: String) -> Result<(), AppError> {
    let form_id = &data.env.attendance_form.id;
    let submission_url = format!("https://docs.google.com/forms/d/{form_id}/formResponse");
    let form_token_input_id = &data.env.attendance_form.token_input_id;
    let form_token_input = &data.env.attendance_form.token_input_value;

    let status = reqwest::Client::new()
        .post(&submission_url)
        .form(&[
            ("emailAddress", email),
            (form_token_input_id.as_str(), form_token_input.to_string()),
        ])
        .send()
        .await?
        .status();

    if status.is_success() {
        Ok(())
    } else {
        bail!("Submission failed")
    }
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

    let success = check_in_with_email(ctx.data(), user.email).await.is_ok();
    if !success {
        ctx.reply_ephemeral("Unable to check in").await?;
        return Ok(());
    };

    ctx.reply_ephemeral(format!("Successfully checked in as {}", user.name))
        .await?;
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
