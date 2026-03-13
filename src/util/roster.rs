use std::{ops::Sub as _, sync::Arc};

use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools as _;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{
    AppError, AppVars, VarsRosterSpreadsheet,
    util::{
        gdrive::GoogleServiceAccount, gforms::submit_google_form, gsheets::get_spreadsheet_range,
    },
};

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RosterSheetRow {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) discord: String,
    pub(crate) committees: Vec<String>,
}

impl RosterSheetRow {
    pub fn is_board(&self) -> bool {
        self.committees.iter().any(|c| c == "board")
    }
}

fn parse_committees_string(committees_text: &str) -> Vec<String> {
    committees_text
        .split(", ")
        .map(|val| val.to_lowercase().replace('_', ""))
        .collect_vec()
}

pub(crate) struct Roster {
    // TODO perhaps change to HashSet and add lookup operations as struct methods instead of standalone functions
    data: Vec<RosterSheetRow>,
    last_updated: DateTime<Utc>,
    service_account: Arc<RwLock<GoogleServiceAccount>>,
    spreadsheet_vars: VarsRosterSpreadsheet,
}

impl Roster {
    pub(crate) fn new(
        spreadsheet_vars: &VarsRosterSpreadsheet,
        service_account: Arc<RwLock<GoogleServiceAccount>>,
    ) -> Roster {
        Roster {
            data: vec![],
            last_updated: DateTime::default(),
            service_account,
            spreadsheet_vars: spreadsheet_vars.clone(),
        }
    }

    async fn refresh_if_needed(&mut self, max_age_mins: i64) -> Result<(), AppError> {
        if self.last_updated > Utc::now().sub(Duration::minutes(max_age_mins)) {
            println!("used previous lookup");
            return Ok(());
        }

        println!("needs new lookup");
        let spreadsheet = &self.spreadsheet_vars;
        self.data = get_spreadsheet_range(
            self.service_account.clone(),
            &spreadsheet.id,
            &spreadsheet.range,
        )
        .await?
        .values
        .into_iter()
        .filter_map(|row| {
            let [name, email, discord, committees] = row.into_iter().collect_array::<4>()?;
            let committees = parse_committees_string(&committees);
            Some(RosterSheetRow {
                name,
                email,
                discord,
                committees,
            })
        })
        .collect_vec();

        self.last_updated = Utc::now();

        Ok(())
    }

    pub(crate) async fn fetch(
        &mut self,
        max_age_mins: i64,
    ) -> Result<&Vec<RosterSheetRow>, AppError> {
        self.refresh_if_needed(max_age_mins).await?;

        Ok(&self.data)
    }

    pub(crate) async fn get_user_from_discord(
        &mut self,
        username: &str,
        force_refresh: bool,
    ) -> Result<Option<RosterSheetRow>, AppError> {
        #[expect(clippy::bool_to_int_with_if)]
        self.refresh_if_needed(if force_refresh { 0 } else { 1 })
            .await?;

        let user = self
            .data
            .iter()
            .find(|row| row.discord.eq_ignore_ascii_case(username))
            .cloned();

        Ok(user)
    }

    pub(crate) async fn get_users_from_discord(
        &mut self,
        usernames: &[&str],
        force_refresh: bool,
    ) -> Result<Vec<RosterSheetRow>, AppError> {
        #[expect(clippy::bool_to_int_with_if)]
        self.refresh_if_needed(if force_refresh { 0 } else { 1 })
            .await?;

        let users = self
            .data
            .iter()
            .filter(|&row| usernames.contains(&row.discord.to_lowercase().as_str()))
            .cloned()
            .collect_vec();

        Ok(users)
    }
}

pub(crate) async fn check_in_with_email(
    data: &AppVars,
    email: &str,
    reason: Option<&str>,
) -> Result<(), AppError> {
    let fields = &data.env.attendance_form;

    let mut payload = vec![
        ("emailAddress", email),
        (&fields.token_input_id, &fields.token_input_value),
    ];

    if let Some(reason) = reason {
        payload.push((&fields.event_input_id, reason));
    }

    submit_google_form(&data.http.client, &fields.id, &payload)
        .await
        .map_err(|err| {
            dbg!(err);
            anyhow!("Google Form submission failed. Please check your inputs.")
        })?;

    Ok(())
}
