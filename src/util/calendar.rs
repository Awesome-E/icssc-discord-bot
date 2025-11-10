use crate::{AppError, AppVars};
use crate::server::{ActixData, ExtractedAppData};
use anyhow::{Context, bail};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use itertools::Itertools;
use jsonwebtoken::Header;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    QueryFilter,
};
use serde::{Deserialize, Serialize};
use serenity::all::{CommandInteraction, GuildId, ScheduledEventId, ScheduledEventType};
use serenity::builder::{CreateScheduledEvent, EditScheduledEvent};
use serenity::futures;
use serenity::http::Http;
use std::cmp::max;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AddCalendarInteractionTrigger {
    pub(crate) guild_id: u64,
    pub(crate) interaction_token: String,
    pub(crate) calendar_id: String,
    pub(crate) iat: u64,
    pub(crate) exp: u64,
}

pub(crate) fn generate_add_calendar_link(
    data: &AppVars,
    ixn: &CommandInteraction,
    calendar_id: String,
) -> Result<String, AppError> {
    let root_url = &data.env.app.origin;

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
        data.env.app.jwt_secret.as_bytes(),
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
        #[serde(rename = "dateTime")]
        date_time: DateTime<Utc>,
        #[serde(rename = "timeZone")]
        time_zone: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoogleCalendarEventDetails {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) description: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) html_link: String,
    pub(crate) created: Option<String>,
    pub(crate) updated: Option<String>,
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
    pub(crate) i_cal_uid: String,
    pub(crate) event_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoogleCalendarEventListResponse {
    pub(crate) summary: String,
    pub(crate) description: Option<String>,
    pub(crate) updated: DateTime<Utc>,
    pub(crate) time_zone: String,
    pub(crate) items: Vec<GoogleCalendarEventDetails>,
}

pub(crate) async fn get_calendar_events(
    calendar_id: &str,
    data: &ExtractedAppData,
    access_token: String,
) -> anyhow::Result<GoogleCalendarEventListResponse> {
    let now = Utc::now();
    let max_ahead = now + Duration::weeks(2);

    let result = data.vars.http.client
        .get(format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            calendar_id
        ))
        .query(&[
            ("showDeleted", "true"), /*("updatedMin", "")*/
            ("timeMin", now.to_rfc3339().as_str()),
            ("timeMax", max_ahead.to_rfc3339().as_str()),
            ("singleEvents", "true"),
        ])
        .bearer_auth(access_token)
        .send()
        .await
        .context("Get calendar events")?
        .json::<GoogleCalendarEventListResponse>()
        .await
        .context("Deserialize calendar events");

    if result.is_err() {
        dbg!(&result);
    };

    result
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CreateWebhookResponse {
    pub(crate) resource_id: String,
}

/// Creates a Google Calendar webhook and returns its Resource ID
pub(crate) async fn create_webhook(
    data: &ActixData,
    calendar_id: String,
    webhook_id: String,
    access_token: String,
) -> anyhow::Result<String> {
    let app_url = &data.vars.env.app.origin;
    let resp = data.vars.http.client
        .post(format!(
            "https://www.googleapis.com/calendar/v3/calendars/{calendar_id}/events/watch"
        ))
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "id": webhook_id.clone(),
            "type": String::from("webhook"),
            "address": format!("{app_url}/calendar/update?id={webhook_id}"),
            "params": { "ttl": String::from("1209600") } // up to 2 weeks
        }))
        .send()
        .await
        .context("Request to create webhook")?
        // .text()
        .json::<CreateWebhookResponse>()
        .await
        .context("parse create webhook response");

    match resp {
        Ok(resp) => Ok(resp.resource_id),
        Err(why) => {
            dbg!(&why);
            Err(why)
        }
    }
}

pub(crate) async fn update_discord_events(
    calendar: &entity::server_calendar::Model,
    conn: &DatabaseConnection,
    http: Arc<Http>,
    mut events: GoogleCalendarEventListResponse,
) -> anyhow::Result<()> {
    use entity::server_event;

    // get all server_events db entries
    let stored_events: HashMap<String, server_event::Model> = calendar
        .find_related(server_event::Entity)
        .all(conn)
        .await?
        .into_iter()
        .map(|e| (e.calendar_event_id.clone(), e))
        .collect();

    let mut deleted: Vec<GoogleCalendarEventDetails> = Vec::new();
    let mut updated: Vec<GoogleCalendarEventDetails> = Vec::new();
    let mut created: Vec<GoogleCalendarEventDetails> = Vec::new();

    while let Some(event) = events.items.pop() {
        match event.status.as_deref() {
            Some("cancelled") => deleted.push(event),
            _ if stored_events.contains_key(&event.id) => updated.push(event),
            _ => created.push(event),
        }
    }

    // for all events that have the same ID, edit or cancel the event
    for event in &deleted {
        let Some(evt_entry) = stored_events.get(&event.id) else {
            continue;
        };
        let guild_id = GuildId::from(calendar.guild_id as u64);
        let event_id = ScheduledEventId::from(evt_entry.guild_event_id as u64);
        // allow nonexistent deletes to silently fail
        let _ = http.delete_scheduled_event(guild_id, event_id).await;
    }
    // remove all deleted events from db
    let deleted_ids = deleted.iter().map(|e| &e.id).collect_vec();
    server_event::Entity::delete_many()
        .filter(server_event::Column::GuildId.eq(calendar.guild_id))
        .filter(server_event::Column::GuildEventId.is_in(deleted_ids));

    let now = Utc::now().add(Duration::seconds(30)).to_utc();
    // changed events => update discord only
    for event in updated {
        let mut payload = EditScheduledEvent::new().name(event.summary.clone());
        if let Some(desc) = &event.description {
            payload = payload.description(desc);
        };

        // by definition of `updated`, it's in stored_events
        let curr_evt_entry = stored_events.get(&event.id).unwrap();
        // TODO handle cases where event has already finished or was deleted by user or completed
        let curr_event = http
            .get_scheduled_event(
                GuildId::from(calendar.guild_id as u64),
                ScheduledEventId::from(curr_evt_entry.guild_event_id as u64),
                false,
            )
            .await;

        let mut move_to_creates = async |event: GoogleCalendarEventDetails,
                                         entry: &server_event::Model|
               -> anyhow::Result<()> {
            created.push(event);
            // allow nonexistent deletions to silently fail
            let _ = server_event::Entity::delete(entry.clone().into_active_model())
                .exec(conn)
                .await;
            Ok(())
        };

        let curr_event = match curr_event {
            Ok(ev) => match ev.end_time {
                Some(end) if end > now.into() => ev,
                Some(_) => {
                    move_to_creates(event, curr_evt_entry).await?;
                    continue;
                }
                None => ev,
            },
            _ => {
                move_to_creates(event, curr_evt_entry).await?;
                continue;
            }
        };

        // Editable if the event on Discord has not started yet
        if let GoogleCalendarEventTime::DateAndTime { date_time, .. } = event.start
            && curr_event.start_time > now.into()
        {
            let new_start = max(date_time, now);
            payload = payload.start_time(new_start);
            if let GoogleCalendarEventTime::DateAndTime { date_time, .. } = event.end {
                let new_end = max(new_start, date_time);
                payload = payload.end_time(new_end);
            };
        };

        http.edit_scheduled_event(
            GuildId::from(calendar.guild_id as u64),
            curr_event.id,
            &payload,
            Some("Sync from Google Calendar"),
        )
        .await?;
    }

    println!("Starting to add events...");

    // added events => db and discord
    let pending_db_entries = created
        .into_iter()
        .map(async |event| {
            // for now, ignore all day events
            let GoogleCalendarEventTime::DateAndTime { date_time, .. } = event.start else {
                return None;
            };
            let start = max(date_time, now);

            let location = event.location.as_deref().unwrap_or("Unknown Location 😱");
            let mut payload =
                CreateScheduledEvent::new(ScheduledEventType::External, event.summary, start)
                    .location(location);
            if let Some(desc) = event.description {
                payload = payload.description(desc);
            };
            if let GoogleCalendarEventTime::DateAndTime { date_time, .. } = event.end {
                let end = max(start, date_time);
                payload = payload.end_time(end);
            }

            let discord_event = http
                .create_scheduled_event(
                    GuildId::from(calendar.guild_id as u64),
                    &payload,
                    "Sync from Google Calendar".into(),
                )
                .await;
            let discord_event = match discord_event {
                Ok(ev) => ev,
                Err(why) => {
                    dbg!(why);
                    return None;
                }
            };
            let db_entry = server_event::ActiveModel {
                guild_id: ActiveValue::set(discord_event.guild_id.into()),
                calendar_id: ActiveValue::set(calendar.calendar_id.clone()),
                calendar_event_id: ActiveValue::set(event.id),
                guild_event_id: ActiveValue::set(discord_event.id.into()),
            };
            Some(db_entry)
        })
        .collect_vec();

    dbg!(pending_db_entries.len());

    let pending_db_entries = futures::future::join_all(pending_db_entries)
        .await
        .into_iter()
        .filter_map(|e| e)
        .collect_vec();

    dbg!(&pending_db_entries);

    let db_res = server_event::Entity::insert_many(pending_db_entries)
        .exec(conn)
        .await;

    // For all events that do not exist
    match db_res {
        Ok(_) => Ok(()),
        Err(why) => {
            dbg!(&why);
            bail!("db error")
        }
    }
}
