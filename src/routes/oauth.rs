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
    use actix_web::http::StatusCode;
    use actix_web::{HttpRequest, HttpResponse, Responder, get, web};
    use anyhow::Context;
    use jsonwebtoken::Validation;
    use serde::{Deserialize, Serialize};
    // use uuid::Uuid;

    // JS will give us the query params unchanged
    #[derive(Debug, Deserialize)]
    struct OAuthCbGoogQuery {
        error: Option<String>,
        code: Option<String>,
        state: String,
    }

    #[derive(Debug, Deserialize)]
    struct GoogleExchangeResponse {
        access_token: String,
        // expires_in: usize,
        // scope: String,
        // always Bearer, for now (https://developers.google.com/identity/protocols/oauth2/web-server)
        // token_type: String,
        refresh_token: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct GoogleUserInfoResponse {
        name: String,
        picture: String,
        email: String,
        id: String,
    }

    #[get("/google")]
    async fn google(
        info: web::Query<OAuthCbGoogQuery>,
        data: ExtractedAppData,
        req: HttpRequest,
    ) -> Result<impl Responder> {
        // client has their "correct state" in the signed cookie
        let cookie_value = match req.cookie("oauth_state") {
            None => return Ok(HttpResponse::build(StatusCode::BAD_REQUEST).body("no state")),
            Some(state) => state.value().to_owned(),
        };

        let OAuthCbGoogQuery {
            code: Some(code),
            error: Option::None,
            state,
            ..
        } = info.into_inner()
        else {
            return Ok(HttpResponse::build(StatusCode::BAD_REQUEST).body("no code or error"));
        };
        let query_state = state;

        let token = jsonwebtoken::decode::<super::GoogleOAuthJWT>(
            &cookie_value,
            &data.jwt_keys.1,
            &Validation::default(),
        )?;

        if token.claims.state != query_state {
            return Ok(HttpResponse::build(StatusCode::BAD_REQUEST).body("state mismatch"));
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
            Err(e) if e.is_status() => {
                return Ok(HttpResponse::build(StatusCode::BAD_REQUEST)
                    .body("goog did not accept exchange"));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(e))?;
            }
            Ok(response) => response,
        }
        .json::<GoogleExchangeResponse>()
        .await
        .context("parse exchange response")?;

        dbg!(&exchange_response.access_token);
        dbg!(&exchange_response.refresh_token);

        Ok(HttpResponse::build(StatusCode::OK).body("hi"))
    }
}
