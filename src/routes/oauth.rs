use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub(crate) struct OAuth {
    pub(crate) frontend_url: String,
    pub(crate) google: GoogleOAuthConfig,
}

#[derive(Clone, Debug)]
pub(crate) struct GoogleOAuthConfig {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
}

#[derive(Serialize, Deserialize)]
struct GoogleOAuthJWT {
    // this is supposed to be a UUIDv4
    state: String,
    // UTC timestamp
    exp: usize,
}

pub(crate) mod start {
    use crate::server::ExtractedAppData;

    use super::*;
    // use crate::ExtractedAppData;
    use crate::server::Result;
    use actix_web::cookie::time::UtcDateTime;
    use actix_web::cookie::{Cookie, SameSite};
    use actix_web::{HttpResponse, Responder, cookie, get, web};
    use anyhow::Context;
    use jsonwebtoken::Header;
    use std::ops::Add;

    #[derive(Debug, Deserialize)]
    struct Query {
        interaction: String,
    }

    #[get("/google")]
    async fn google(info: web::Query<Query>, data: ExtractedAppData) -> Result<impl Responder> {
        let state = uuid::Uuid::new_v4();

        let redirect_uri = format!("{}/oauth/cb/google", data.oauth.frontend_url);

        let goog_request = data
            .client
            .get("https://accounts.google.com/o/oauth2/v2/auth")
            .query(&[
                ("client_id", &*data.oauth.google.client_id),
                ("redirect_uri", &*redirect_uri),
                ("response_type", "code"),
                ("state", state.to_string().as_str()),
                (
                    "scope",
                    "https://www.googleapis.com/auth/calendar.events.readonly",
                ),
                ("access_type", "offline"),
                ("prompt", "consent"),
            ])
            .build()?;

        let jwt = GoogleOAuthJWT {
            state: state.to_string(),
            exp: UtcDateTime::now()
                .add(std::time::Duration::from_secs(5 * 60))
                .unix_timestamp() as usize,
        };

        let encoded = jsonwebtoken::encode(&Header::default(), &jwt, &data.jwt_keys.0)
            .context("build JWT token")?;

        Ok(HttpResponse::Found()
            .insert_header(("Location", goog_request.url().as_str()))
            .cookie(
                Cookie::build("oauth_state", encoded.to_string())
                    .max_age(cookie::time::Duration::minutes(10))
                    .same_site(SameSite::Lax) // not defaulted on firefox and safari
                    .path("/")
                    .finish(),
            )
            .cookie(
                Cookie::build("interaction", info.interaction.clone())
                    .max_age(cookie::time::Duration::minutes(10))
                    .same_site(SameSite::Lax)
                    .path("/")
                    .finish(),
            )
            .finish())
    }
}

pub(crate) mod cb {
    // use crate::ExtractedAppData;
    // use actix_session::Session;
    use crate::server::{ExtractedAppData, Result};
    use crate::util;
    use crate::util::calendar::{AddCalendarInteractionTrigger, create_webhook};
    use actix_web::cookie::{Cookie, SameSite};
    use actix_web::http::StatusCode;
    use actix_web::{HttpRequest, HttpResponse, Responder, get, web};
    use anyhow::{Context, anyhow, bail};
    use chrono::{Duration, Utc};
    use jsonwebtoken::Validation;
    use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
    use serde::{Deserialize, Serialize};

    // JS will give us the query params unchanged
    #[derive(Debug, Deserialize)]
    struct OAuthCbGoogQuery {
        error: Option<String>,
        code: Option<String>,
        state: String,
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct GoogleExchangeResponse {
        pub(crate) access_token: String,
        pub(crate) expires_in: i64,
        // scope: String,
        // always Bearer, for now (https://developers.google.com/identity/protocols/oauth2/web-server)
        // token_type: String,
        pub(crate) refresh_token: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct GoogleUserInfoResponse {
        name: String,
        picture: String,
        email: String,
        id: String,
    }

    async fn get_oauth_tokens(
        info: web::Query<OAuthCbGoogQuery>,
        data: &ExtractedAppData,
        req: &HttpRequest,
    ) -> anyhow::Result<GoogleExchangeResponse> {
        // client has their "correct state" in the signed cookie
        let cookie_value = match req.cookie("oauth_state") {
            None => return Err(anyhow!("Missing OAuth State")),
            Some(state) => state.value().to_owned(),
        };

        let OAuthCbGoogQuery {
            code: Some(code),
            error: Option::None,
            state,
            ..
        } = info.into_inner()
        else {
            return Err(anyhow!("OAuth code was missing"));
        };
        let query_state = state;

        let token = jsonwebtoken::decode::<super::GoogleOAuthJWT>(
            &cookie_value,
            &data.jwt_keys.1,
            &Validation::default(),
        )?;

        if token.claims.state != query_state {
            return Err(anyhow!("OAuth state mismatch"));
        }

        let redirect_uri = format!("{}/oauth/cb/google", data.oauth.frontend_url);
        let exchange_response = match data
            .client
            .post("https://oauth2.googleapis.com/token")
            .query(&[
                ("client_id", &*data.oauth.google.client_id),
                ("client_secret", &*data.oauth.google.client_secret),
                ("code", &*code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", &*redirect_uri),
            ])
            .header("Content-Length", "0")
            .send()
            .await
        {
            Err(e) => {
                return Err(anyhow!("OAuth exchange error: {}", e))?;
            }
            Ok(response) => response,
        }
        .json::<GoogleExchangeResponse>()
        .await
        .context("parse exchange response")?;

        Ok(exchange_response)
    }

    fn get_interaction(
        data: &ExtractedAppData,
        req: &HttpRequest,
    ) -> anyhow::Result<AddCalendarInteractionTrigger> {
        let Some(ixn_cookie) = req.cookie("interaction") else {
            return Err(anyhow!("No interaction found"));
        };

        let decoded = jsonwebtoken::decode::<AddCalendarInteractionTrigger>(
            ixn_cookie.value(),
            &data.jwt_keys.1,
            &Validation::default(),
        );

        let ixn_data = match decoded {
            Ok(ixn_data) => ixn_data,
            Err(why) => {
                dbg!(why);
                bail!("Bad interaction data")
            }
        };

        Ok(ixn_data.claims)
    }

    #[get("/google")]
    async fn google(
        info: web::Query<OAuthCbGoogQuery>,
        data: ExtractedAppData,
        req: HttpRequest,
    ) -> Result<impl Responder> {
        let exchange_response = get_oauth_tokens(info, &data, &req).await?;

        let Some(refresh_token) = exchange_response.refresh_token else {
            return Ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Failed to get refresh token"));
        };

        let interaction = get_interaction(&data, &req)?;

        // check member permission to add events - TODO later
        // let original_ixn = data
        //     .http_action
        //     .get_original_interaction_response(&interaction.interaction_token)
        //     .await
        //     .context("Failed to get original interaction")?;
        //
        // let author_id = original_ixn.author.id;
        // let has_event_perms = data
        //     .http_action
        //     .get_member(GuildId::from(interaction.guild_id.clone()), author_id)
        //     .await
        //     .map(|member| {
        //         let Some(permissions) = member.permissions else {
        //             return false;
        //         };
        //         permissions.create_events()
        //             || permissions.manage_events()
        //             || permissions.administrator()
        //     })?;
        // if !has_event_perms {
        //     return Ok(HttpResponse::build(StatusCode::FORBIDDEN)
        //         .body("You do not have permission to create events"));
        // }

        let conn = {
            // DO NOT ACTUALLY DO THIS HERE LMAO
            let db_url = std::env::var("DATABASE_URL").expect("need postgres URL!");
            sea_orm::Database::connect(&db_url).await?
        };

        let None = entity::server_calendar::Entity::find()
            .filter(entity::server_calendar::Column::CalendarId.eq(interaction.calendar_id.clone()))
            .one(&conn)
            .await
            .context("Search calendars")?
        else {
            return Ok(HttpResponse::build(StatusCode::CONFLICT)
                .body("This calendar is already being watched in this server"));
        };

        // pull events from GCal
        let events = util::calendar::get_calendar_events(
            &interaction.calendar_id,
            &data,
            exchange_response.access_token.clone(),
        )
        .await?;

        let webhook_id = uuid::Uuid::new_v4().to_string();

        // Create the Google Calendar Webhook
        let resource_id = create_webhook(
            &data.client,
            interaction.calendar_id.clone(),
            webhook_id.clone(),
            exchange_response.access_token.clone(),
        )
        .await?;

        let expires = Utc::now() + Duration::seconds(exchange_response.expires_in);
        let server_cal_model = entity::server_calendar::ActiveModel {
            guild_id: ActiveValue::set(interaction.guild_id as i64),
            calendar_id: ActiveValue::set(interaction.calendar_id),
            calendar_name: ActiveValue::set(events.summary.clone()),
            webhook_id: ActiveValue::set(webhook_id),
            access_token: ActiveValue::set(exchange_response.access_token),
            access_expires: ActiveValue::set(expires.naive_utc()),
            refresh_token: ActiveValue::set(refresh_token),
            webhook_last_updated: Default::default(),
            // TODO make this non-optional
            webhook_g_cal_resource_id: ActiveValue::set(Some(resource_id)),
        };

        entity::server_calendar::Entity::insert(server_cal_model)
            .exec(&conn)
            .await?;

        // let ixn_token = ixn_data.claims.interaction_token;
        let interaction_update = serenity::all::EditInteractionResponse::new()
            .content(format!("Added new calendar: `{}`", events.summary));
        data.http_action
            .edit_original_interaction_response(
                &interaction.interaction_token,
                &interaction_update,
                vec![],
            )
            .await?;

        Ok(HttpResponse::build(StatusCode::OK)
            .cookie(
                Cookie::build("interaction", "")
                    .same_site(SameSite::Lax)
                    .path("/")
                    .finish(),
            )
            .body("Successfully added calendar!"))
    }
}
