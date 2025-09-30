use anyhow::{Error, Result, bail};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{BotError, Context, util::ContextExtras};

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
struct SheetsResp {
    values: Vec<[String; 3]>,
}

async fn get_gsheets_token() -> Result<TokenResponse, BotError> {
    let key_id =
        std::env::var("ICSSC_SERVICE_ACC_KEY_ID").expect("Need ICSSC Service Account Key ID");
    let key_email =
        std::env::var("ICSSC_SERVICE_ACC_KEY_EMAIL").expect("Need ICSSC Service Account Key Email");
    let key_pem =
        std::env::var("ICSSC_SERVICE_ACC_KEY_PEM").expect("Need ICSSC Service Account Key PEM");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let now = now.as_secs();

    let claims = Claims {
        iss: key_email.clone(),
        sub: key_email,
        scope: "https://www.googleapis.com/auth/spreadsheets.readonly".to_owned(),
        aud: "https://oauth2.googleapis.com/token".to_owned(),
        exp: now + 3600,
        iat: now,
    };

    let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(key_id.to_owned());

    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(key_pem.as_bytes())?,
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
    access_token: &String,
    username: String,
) -> Result<Option<SheetsRow>, BotError> {
    let spreadsheet_id =
        std::env::var("ICSSC_ROSTER_SPREADSHEET_ID").expect("Spreadsheet ID not defined");
    let spreadsheet_range =
        std::env::var("ICSSC_ROSTER_SPREADSHEET_RANGE").expect("Spreadsheet Range not defined");

    let resp = reqwest::Client::new()
        .get(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{spreadsheet_range}"
        ))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<SheetsResp>()
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

async fn check_in_with_email(email: String) -> Result<(), BotError> {
    let form_id = std::env::var("ICSSC_ROSTER_FORM_ID").expect("ICSSC Roster Form ID Missing");
    let submission_url = format!("https://docs.google.com/forms/d/{form_id}/formResponse");
    let form_token_input_id = std::env::var("ICSSC_ROSTER_FORM_TOK_INPUT_ID")
        .expect("ICSSC Roster Form Input ID Missing");
    let form_token_input =
        std::env::var("ICSSC_ROSTER_FORM_TOKEN").expect("ICSSC Roster Form Token Missing");

    let status = reqwest::Client::new()
        .post(&submission_url)
        .form(&[
            ("emailAddress", email),
            (form_token_input_id.as_str(), form_token_input),
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
pub(crate) async fn check_in(ctx: Context<'_>) -> Result<(), Error> {
    let Ok(TokenResponse { access_token }) = get_gsheets_token().await else {
        ctx.reply_ephemeral("Unable to find who you are :(").await?;
        return Ok(());
    };

    ctx.defer_ephemeral().await?;

    let username = &ctx.author().name;
    let Ok(Some(user)) = get_user_from_discord(&access_token, username.to_string()).await else {
        ctx.reply_ephemeral(
            "\
Cannot find a matching internal member. Double check that your \
Discord username on the internal roster is correct.",
        )
        .await?;
        return Ok(());
    };

    let success = check_in_with_email(user.email).await.is_ok();
    if !success {
        ctx.reply_ephemeral("Unable to check in").await?;
        return Ok(());
    };

    ctx.reply_ephemeral(format!("Successfully checked in as {}", user.name))
        .await?;
    Ok(())
}
