use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{EncodingKey, Header, encode};

use crate::{AppError, AppVars};


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
    pub(crate) access_token: String,
    // Can also read token_type: String and expires_in: u64
}

// TODO consolidate all google sheets helpers
#[derive(Debug, Deserialize)]
pub(crate) struct SheetsResponse {
    pub(crate) values: Vec<Vec<String>>,
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

pub(crate) async fn get_spreadsheet_range(data: &AppVars, sheet_id: &str, range: &str, access_token: Option<&str>) -> anyhow::Result<SheetsResponse> {
    let access_token = match access_token {
        Some(tok) => tok,
        None => &get_gsheets_token(data).await?.access_token,
    };

    let resp = reqwest::Client::new()
        .get(format!("https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}/values/{range}"))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<SheetsResponse>()
        .await?;

    Ok(resp)
}
