use actix_web::cookie::time::UtcDateTime;
use anyhow::Context;
use chrono::{Date, DateTime, NaiveDate, NaiveDateTime, Utc};
use jsonwebtoken::Header;
use serde::{Deserialize, Serialize};
use serenity::all::CommandInteraction;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::BotError;
use crate::server::ExtractedAppData;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AddCalendarInteractionTrigger {
    pub(crate) guild_id: u64,
    pub(crate) interaction_token: String,
    pub(crate) calendar_id: String,
    pub(crate) iat: u64,
    pub(crate) exp: u64,
}

pub(crate) fn generate_add_calendar_link(
    ixn: &CommandInteraction,
    calendar_id: String,
) -> Result<String, BotError> {
    let root_url = std::env::var("RAILWAY_PUBLIC_DOMAIN").context("Missing Domain")?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let jwt = AddCalendarInteractionTrigger {
        guild_id: ixn
            .guild_id
            .map(|v| v.get())
            .context("Generate oauth link => guild id")?,
        interaction_token: ixn.token.clone(),
        calendar_id,
        iat: now,
        exp: now + 60 * 10,
    };

    let encoding_key = jsonwebtoken::EncodingKey::from_secret(
        std::env::var_os("JWT_SECRET")
            .context("Missing JWT_SECRET")?
            .as_encoded_bytes(),
    );

    let encoded = jsonwebtoken::encode(&Header::default(), &jwt, &encoding_key)
        .context("Generate oauth link => encode interaction")?;

    let url = reqwest::Client::new()
        .get(format!("{root_url}/oauth/start/google"))
        .query(&[
            ("interaction", encoded), // imo, not worth db logic to store tokens that are only valid for 15 mins
        ])
        .build()
        .context("Generate oauth link => build url")?
        .url()
        .to_string();

    Ok(url)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum GoogleCalendarEventTime {
    DayOnly {
        date: NaiveDate,
    },
    DateAndTime {
        dateTime: DateTime<Utc>,
        timeZone: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GoogleCalendarEventDetails {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) description: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) htmlLink: String,
    pub(crate) created: String,
    pub(crate) updated: String,
    pub(crate) summary: String,
    pub(crate) location: Option<String>,
    // creator: { email: String }
    // "organizer": {
    //   "email": "c_qs7rsl2doup84lrplnbpb0dnvg@group.calendar.google.com",
    //   "displayName": "ICSSC External Calendar",
    //   "self": true
    // },
    pub(crate) start: GoogleCalendarEventTime,
    pub(crate) end: GoogleCalendarEventTime,
    pub(crate) iCalUID: String,
    pub(crate) eventType: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GoogleCalendarEventListResponse {
    pub(crate) summary: String,
    pub(crate) description: Option<String>,
    pub(crate) updated: DateTime<Utc>,
    pub(crate) timeZone: String,
    pub(crate) items: Vec<GoogleCalendarEventDetails>,
}

pub(crate) async fn get_calendar_events(
    calendar_id: &str,
    data: &ExtractedAppData,
    access_token: String,
) -> anyhow::Result<GoogleCalendarEventListResponse> {
    data.client
        .get(format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            calendar_id
        ))
        .query(&[
            ("showDeleted", "true"), /*("updatedMin", "")*/
            ("timeMin", "2025-09-01T00:00:00Z"),
            ("timeMax", "2025-11-01T00:00:00Z"),
        ])
        .bearer_auth(access_token)
        .send()
        .await
        .context("Get calendar events")?
        .json::<GoogleCalendarEventListResponse>()
        .await
        .context("Deserialize calendar events")
}
