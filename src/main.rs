mod handler;
mod matchy;
mod spottings;
mod util;
mod setup;

use crate::setup::{create_bot_framework_options, framework_setup};
use serenity::all::{GatewayIntents};
use serenity::{Client};
use std::env;
use std::ops::BitAnd;

struct BotVars {
    db: sea_orm::DatabaseConnection,
}

#[tokio::main]
async fn main() {
    setup::load_env();

    let framework = poise::Framework::<BotVars, BotError>::builder()
        .options(create_bot_framework_options())
        .setup(framework_setup)
        .build();

    let token = env::var("ICSSC_DISCORD_TOKEN").expect("no discord token set");
    let mut client = Client::builder(
        &token,
        GatewayIntents::non_privileged()
            .bitand(GatewayIntents::GUILD_MEMBERS)
            .bitand(GatewayIntents::MESSAGE_CONTENT),
    )
    .event_handler(handler::ICSSpottingsCouncilEventHandler)
    .framework(framework)
    .await
    .expect("couldn't make client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

type BotError = anyhow::Error;
type Context<'a> = poise::Context<'a, BotVars, BotError>;
