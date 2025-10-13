use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use actix_web::{App, HttpServer, ResponseError, web};
use anyhow::Context;
use serenity::all::Http;

use crate::routes::{
    self,
    oauth::{self, GoogleOAuthConfig, OAuth},
};

#[derive(Clone)]
pub(crate) struct AppData {
    pub(crate) client: reqwest::Client,
    pub(crate) oauth: OAuth,
    pub(crate) jwt_keys: (jsonwebtoken::EncodingKey, jsonwebtoken::DecodingKey),
    pub(crate) http_action: Arc<Http>,
    // db: sea_orm::DatabaseConnection,
}
pub(crate) type ExtractedAppData = web::Data<AppData>;

#[repr(transparent)]
#[derive(Debug)]
pub(crate) struct AnyhowBridge(anyhow::Error);

impl Display for AnyhowBridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for AnyhowBridge
where
    T: Into<anyhow::Error>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

pub(crate) type Result<T> = std::result::Result<T, AnyhowBridge>;

impl ResponseError for AnyhowBridge {}

pub(crate) async fn run(http_action: Arc<Http>) -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .unwrap_or(String::from("2509"))
        .parse::<u16>()
        .context("$PORT not valid u16 port")?;

    let jwt_secret = std::env::var_os("JWT_SECRET").context("Missing JWT_SECRET")?;
    let server_url =
        std::env::var("RAILWAY_PUBLIC_DOMAIN").context("Missing RAILWAY_PUBLIC_DOMAIN")?;
    let oauth_client_id =
        std::env::var("GOOGLE_OAUTH_CLIENT_ID").context("Missing GOOGLE_OAUTH_CLIENT_ID")?;
    let oauth_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
        .context("Missing GOOGLE_OAUTH_CLIENT_SECRET")?;

    let app_data = AppData {
        client: reqwest::Client::new(),
        oauth: OAuth {
            frontend_url: server_url,
            google: GoogleOAuthConfig {
                client_id: oauth_client_id,
                client_secret: oauth_secret,
            },
        },
        jwt_keys: (
            jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_encoded_bytes()),
            jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_encoded_bytes()),
        ),
        http_action,
    };

    let server = {
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(app_data.clone()))
                .service(
                    web::scope("/calendar").service(routes::calendar::webhook::update_calendar),
                )
                .service(web::scope("/oauth/start").service(oauth::start::google))
                .service(web::scope("/oauth/cb").service(oauth::cb::google))
                .service(
                    web::scope("/webhook").service(routes::webhook::add_event_test::add_event_test),
                )
        })
        .bind(("::", port))
        .with_context(|| format!("failed to bind to port {port}"))
    }
    .expect("Start server");

    println!("Listening on port {port}...");

    Ok(server.run().await?)
}
