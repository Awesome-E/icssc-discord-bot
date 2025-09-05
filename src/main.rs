mod config;
mod create_pairing;
mod discord_helpers;
mod helpers;
mod matching;
mod send_pairing;
mod types;
use crate::create_pairing::create_pairing;
use crate::helpers::handle_error;
use crate::send_pairing::send_pairing;
use poise::serenity_prelude as serenity;
use std::sync::Arc;

pub const ROLE_NAME: &str = "matchy-meetups";

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            on_error: |err| Box::pin(handle_error(err)),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("~".into()),
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    std::time::Duration::from_secs(3600),
                ))),
                case_insensitive_commands: true,
                ..Default::default()
            },
            commands: vec![create_pairing(), send_pairing()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(())
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
