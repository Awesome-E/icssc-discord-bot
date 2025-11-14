use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::bail;
use itertools::Itertools;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::{AppError, AppVars};

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    // Can also read token_type: String and expires_in: u64
}

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
pub(crate) struct SheetsRow {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) discord: String,
}

#[derive(Debug, Deserialize)]
struct RosterSheetsResp {
    values: Vec<[String; 3]>,
}

pub(crate) async fn get_gsheets_token(data: &AppVars) -> Result<TokenResponse, AppError> {
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
    )?;

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

async fn get_roster_rows(
    data: &AppVars,
    access_token: Option<&String>,
) -> Result<RosterSheetsResp, AppError> {
    let spreadsheet = &data.env.roster_spreadsheet;
    let access_token = match access_token {
        Some(tok) => tok,
        None => &get_gsheets_token(data).await?.access_token,
    };

    let resp = reqwest::Client::new()
        .get(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet.id, spreadsheet.range
        ))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<RosterSheetsResp>()
        .await?;

    Ok(resp)
}

pub(crate) async fn get_user_from_discord(
    data: &AppVars,
    access_token: &String,
    username: String,
) -> Result<Option<SheetsRow>, AppError> {
    let resp = get_roster_rows(data, Some(access_token)).await?;

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

pub(crate) async fn get_bulk_members_from_roster(
    data: &AppVars,
    usernames: &[String],
) -> Result<Vec<SheetsRow>, AppError> {
    let usernames: HashSet<&String> = usernames.iter().collect();
    let resp = get_roster_rows(data, None).await?;
    let rows = resp
        .values
        .iter()
        .filter_map(|row| {
            let [name, email, discord] = row;

            let found = usernames.contains(&discord.to_lowercase());
            if !found {
                return None;
            }

            Some(SheetsRow {
                name: name.to_string(),
                email: email.to_string(),
                discord: discord.to_string(),
            })
        })
        .collect_vec();

    Ok(rows)
}

pub(crate) async fn check_in_with_email(
    data: &AppVars,
    email: &str,
    reason: Option<String>,
) -> Result<(), AppError> {
    let form_id = &data.env.attendance_form.id;
    let submission_url = format!("https://docs.google.com/forms/d/{form_id}/formResponse");
    let form_token_input_id = &data.env.attendance_form.token_input_id;
    let form_token_input = &data.env.attendance_form.token_input_value;
    let form_event_input_id = &data.env.attendance_form.event_input_id;

    let mut payload = vec![
        ("emailAddress", email.to_string()),
        (form_token_input_id.as_str(), form_token_input.clone()),
    ];

    if let Some(reason) = reason {
        payload.push((form_event_input_id.as_str(), reason));
    };

    let status = reqwest::Client::new()
        .post(&submission_url)
        .form(&payload)
        .send()
        .await?
        .status();

    if status.is_success() {
        Ok(())
    } else {
        bail!("Submission failed")
    }
}
