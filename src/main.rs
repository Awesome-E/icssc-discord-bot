mod handler;
mod matchy;
mod spottings;
mod util;
mod setup;

use crate::setup::{create_bot_framework_options, register_commands};
use clap::ValueHint;
use serenity::all::{GatewayIntents};
use serenity::{Client};
use std::env;
use std::ops::{BitAnd, Deref};
use std::path::PathBuf;

struct BotVarsInner {
    db: sea_orm::DatabaseConnection,
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
                }
            })
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
            |ctx, _ready, framework| Box::pin(async move {
                register_commands(&ctx, &framework).await?;
                Ok(data)
            })
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

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

type BotError = anyhow::Error;
type Context<'a> = poise::Context<'a, BotVars, BotError>;
