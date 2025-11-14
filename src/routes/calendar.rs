pub(crate) mod webhook {
    use crate::routes::oauth::cb::GoogleExchangeResponse;
    use crate::server::ExtractedAppData;
    use crate::util::calendar::{get_calendar_events, update_discord_events};
    use actix_web::{HttpResponse, Responder, post, web};
    use anyhow::{Context, anyhow};
    use chrono::{Duration, Utc};
    use entity::server_calendar;
    use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait,
        IntoActiveModel, QueryFilter,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct UpdateCalendarQuery {
        id: String,
    }

    async fn refresh_access_token(
        data: &ExtractedAppData,
        calendar: &entity::server_calendar::Model,
        conn: &DatabaseConnection,
    ) -> anyhow::Result<String> {
        if calendar.access_expires > chrono::Utc::now().naive_utc() {
            return Ok(calendar.access_token.clone());
        }
        let resp = data
            .vars
            .http
            .client
            .get("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", data.vars.env.google_oauth_client.id.as_str()),
                (
                    "client_secret",
                    data.vars.env.google_oauth_client.secret.as_str(),
                ),
                ("refresh_token", calendar.refresh_token.as_str()),
            ])
            .send()
            .await
            .context("refresh access token")?
            .json::<GoogleExchangeResponse>()
            .await
            .context("Parse response")?;

        let Some(refresh_token) = resp.refresh_token else {
            return Err(anyhow!("Refresh token not present"));
        };

        let expires = Utc::now() + Duration::seconds(resp.expires_in);
        let mut server_cal = calendar.clone().into_active_model();
        server_cal.access_token = ActiveValue::set(resp.access_token.clone());
        server_cal.access_expires = ActiveValue::set(expires.naive_utc());
        server_cal.refresh_token = ActiveValue::set(refresh_token);
        server_cal.update(conn).await?;
        Ok(resp.access_token)
    }

    #[post("/update")]
    async fn update_calendar(
        info: web::Query<UpdateCalendarQuery>,
        data: ExtractedAppData,
        // req: HttpRequest,
    ) -> crate::server::Result<impl Responder> {
        let conn = &data.vars.db;

        let webhook_id = info.id.clone();
        println!("[update] calendar with webhook id: {}", &webhook_id);

        let calendar = entity::server_calendar::Entity::find()
            .filter(entity::server_calendar::Column::WebhookId.eq(webhook_id))
            .one(conn)
            .await?;

        let Some(calendar) = calendar else {
            return Ok(HttpResponse::NotFound().body("No such calendar webhook exists"));
        };

        // get access token (may need to refresh)
        let access_token = refresh_access_token(&data, &calendar, conn)
            .await
            .context("Refresh access token")?;

        // get the events
        let events = get_calendar_events(&calendar.calendar_id, &data, access_token)
            .await
            .context("Failed to get calendar events")?;

        println!("[update] webhook received {} events", events.items.len());

        // handle each event... something like that...
        let update_resp =
            update_discord_events(&calendar, &conn, data.discord_http.clone(), events).await;
        if let Err(why) = update_resp {
            dbg!(&why);
        }

        // update db
        let mut cal_update = calendar.into_active_model();
        cal_update.webhook_last_updated = ActiveValue::set(Utc::now().naive_utc().into());
        server_calendar::Entity::update(cal_update)
            .exec(conn)
            .await?;

        println!("[update] Done updating");
        Ok(HttpResponse::Ok().body("ok"))
    }
}
