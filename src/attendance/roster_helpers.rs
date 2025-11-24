use std::collections::HashSet;

use anyhow::bail;
use itertools::Itertools;
use serde::{Deserialize};

use crate::{AppError, AppVars, util::gsheets::{SheetsResponse, get_gsheets_token, get_spreadsheet_range}};

#[derive(Debug, Deserialize)]
pub(crate) struct RosterSheetRow {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) discord: String,
}

async fn get_roster_rows(
    data: &AppVars,
    access_token: Option<&str>,
) -> Result<SheetsResponse, AppError> {
    let spreadsheet = &data.env.roster_spreadsheet;
    let access_token = match access_token {
        Some(tok) => tok,
        None => &get_gsheets_token(data).await?.access_token,
    };

    let resp = get_spreadsheet_range(data, &spreadsheet.id, &spreadsheet.range, Some(access_token)).await?;

    Ok(resp)
}

pub(crate) async fn get_user_from_discord(
    data: &AppVars,
    access_token: Option<&str>,
    username: String,
) -> Result<Option<RosterSheetRow>, AppError> {
    let resp = get_roster_rows(data, access_token).await?;

    let user = resp
        .values
        .into_iter()
        .filter_map(|row| {
            let row = row.into_iter().collect_array::<3>()?;
            let [name, email, discord] = row;
            Some(RosterSheetRow {
                name: name.to_string(),
                email: email.to_string(),
                discord: discord.to_string(),
            })
        })
        .find(|row| row.discord.to_lowercase() == username);

    Ok(user)
}

pub(crate) async fn get_bulk_members_from_roster(
    data: &AppVars,
    usernames: &[String],
) -> Result<Vec<RosterSheetRow>, AppError> {
    let usernames: HashSet<&String> = usernames.iter().collect();
    let resp = get_roster_rows(data, None).await?;


    let rows = resp
        .values
        .into_iter()
        .filter_map(|row| {
            let [name, email, discord] = row.into_iter().collect_array::<3>()?;

            let found = usernames.contains(&discord.to_lowercase());
            if !found {
                return None;
            }

            Some(RosterSheetRow {
                name,
                email,
                discord,
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
