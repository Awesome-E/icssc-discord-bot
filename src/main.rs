mod attendance;
mod handler;
mod matchy;
mod internal_commands;
mod routes;
mod server;
mod setup;
mod spottings;
mod util;

use crate::setup::{create_bot_framework_options, register_commands};
use anyhow::Context as _;
use clap::ValueHint;
use serenity::Client;
use serenity::all::GatewayIntents;
use std::env;
use std::ops::{BitAnd, Deref};
use std::path::PathBuf;

struct BotVarsInner {
    db: sea_orm::DatabaseConnection,
    icssc_guild_id: u64,
    matchy_channel_id: u64,
}

#[derive(Clone)]
struct BotVars {
    inner: std::sync::Arc<BotVarsInner>,
}

impl Deref for BotVars {
    type Target = BotVarsInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BotVars {
    async fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(BotVarsInner {
                db: {
                    let db_url = env::var("DATABASE_URL").expect("need postgres URL!");
                    sea_orm::Database::connect(&db_url).await.unwrap()
                },
                icssc_guild_id: env::var("ICSSC_GUILD_ID")
                    .expect("need ICSSC_GUILD_ID")
                    .parse::<_>()
                    .expect("ICSSC_GUILD_ID must be valid u64"),
                matchy_channel_id: env::var("ICSSC_MATCHY_CHANNEL_ID")
                    .expect("need ICSSC_MATCHY_CHANNEL_ID")
                    .parse::<_>()
                    .expect("ICSSC_MATCHY_CHANNEL_ID must be valid u64"),
            }),
        }
    }
}

#[tokio::main]
async fn main() {
    let cmd = clap::command!("icssc-discord-bot")
        .about("The somewhat official Discord bot for ICS Student Council")
        .arg(
            clap::arg!(["config"] ".env file path")
                .value_parser(clap::value_parser!(PathBuf))
                .value_hint(ValueHint::FilePath)
                .default_value(".env"),
        );

    let args = cmd.get_matches();

    setup::load_env(args);

    let data = BotVars::new().await;

    let framework = poise::Framework::<BotVars, BotError>::builder()
        .options(create_bot_framework_options())
        .setup({
            let data = data.clone();
            |ctx, _ready, framework| {
                Box::pin(async move {
                    register_commands(ctx, framework).await?;
                    Ok(data)
                })
            }
        })
        .build();

    let token = env::var("ICSSC_DISCORD_TOKEN").expect("no discord token set");
    let mut client = Client::builder(
        &token,
        GatewayIntents::non_privileged()
            .bitand(GatewayIntents::GUILD_MEMBERS)
            .bitand(GatewayIntents::MESSAGE_CONTENT),
    )
    .event_handler(handler::LaikaEventHandler { data })
    .framework(framework)
    .await
    .expect("couldn't make client");

    let http_action = client.http.clone();

    let serenity_task = async move {
        client.start().await.context("start serenity")?;
        anyhow::Result::<()>::Ok(())
    };

    let actix_task = async move {
        crate::server::run(http_action).await.context("start axtix")?;
        anyhow::Result::<()>::Ok(())
    };

    tokio::select! {
        biased;

        _ = tokio::signal::ctrl_c() => {
            println!("SIGINT, going down");
        }

        _ = serenity_task => {
            println!("serenity has stopped")
        }

        _ = actix_task => {
            println!("actix has stopped")
        }
    }
}

type BotError = anyhow::Error;
type Context<'a> = poise::Context<'a, BotVars, BotError>;
