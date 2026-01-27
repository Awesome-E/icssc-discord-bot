use serde::Deserialize;

use crate::{
    AppError, AppVars,
    util::gdrive::{TokenResponse, get_google_oauth_token},
};

#[derive(Debug, Deserialize)]
pub(crate) struct SheetsResponse {
    pub(crate) values: Vec<Vec<String>>,
}

pub(crate) async fn get_gsheets_token(data: &AppVars) -> Result<TokenResponse, AppError> {
    let resp = get_google_oauth_token(
        data,
        "https://www.googleapis.com/auth/spreadsheets.readonly",
    )
    .await?;
    Ok(resp)
}

pub(crate) async fn get_spreadsheet_range(
    data: &AppVars,
    sheet_id: &str,
    range: &str,
    access_token: Option<&str>,
) -> anyhow::Result<SheetsResponse> {
    let access_token = match access_token {
        Some(tok) => tok,
        None => &get_gsheets_token(data).await?.access_token,
    };

    let resp = reqwest::Client::new()
        .get(format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{sheet_id}/values/{range}"
        ))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<SheetsResponse>()
        .await?;

    Ok(resp)
}
