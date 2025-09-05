mod handler;
mod matchy;
mod spottings;
mod util;
mod setup;

use crate::setup::framework_setup;
use crate::util::ContextExtras;
use poise::{FrameworkError, FrameworkOptions};
use serenity::all::{GatewayIntents, GuildId};
use serenity::{Client, FutureExt};
use std::env;
use std::ops::BitAnd;

struct BotVars {
    db: sea_orm::DatabaseConnection,
}

const ICSSC_SERVER: u64 = 760915616793755669;
const ALLOWED_CHANNELS: &[u64] = &[1328907402321592391, 1338632123929591970];

#[tokio::main]
async fn main() {
    setup::load_env();

    let framework = poise::Framework::<BotVars, BotError>::builder()
        .options(FrameworkOptions {
            on_error: |error| {
                Box::pin(async move {
                    println!("Error: {}", error);

                    let Some(ctx) = error.ctx() else { return };
                    let error_res = match error {
                        FrameworkError::Command {
                            error: wrapped_error,
                            ..
                        } => {
                            ctx.reply_ephemeral(format!(
                                "An unexpected error occurred: {:?}",
                                wrapped_error
                            ))
                            .await
                        }
                        _ => ctx.reply_ephemeral("An unknown error occurred").await,
                    };
                    if let Err(e) = error_res {
                        println!(
                            "A further error occurred sending the error message to discord: {:?}",
                            e
                        )
                    }
                })
            },
            commands: vec![
                matchy::create_pairing::create_pairing(),
                matchy::send_pairing::send_pairing(),
                spottings::meta::ping(),
                spottings::snipe::snipe(),
                spottings::leaderboard::leaderboard(),
                spottings::privacy::opt_out(),
            ],
            command_check: Some(|ctx| {
                async move {
                    Ok(ctx.guild_id() != Some(GuildId::from(ICSSC_SERVER))
                        || ALLOWED_CHANNELS.contains(&ctx.channel_id().into()))
                }
                .boxed()
            }),
            ..Default::default()
        })
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
