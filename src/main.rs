mod commands;
mod handler;
mod model;
mod schema;
mod util;

use crate::commands::meta;
use clap::ValueHint;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use itertools::Itertools;
use pluralizer::pluralize;
use poise::{Command, FrameworkOptions, PrefixFrameworkOptions};
use serenity::all::{GatewayIntents, GuildId};
use serenity::Client;
use std::env;
use std::ops::BitAnd;
use std::path::PathBuf;

struct BotVars {
    db_pool: Pool<AsyncPgConnection>,
}

#[tokio::main]
async fn main() {
    let cmd = clap::command!("ics-spottings-council")
        .about("Did you know that ICSSC also stands for ICS Spottings Council?")
        .arg(
            clap::arg!(<"config"> ".env file path")
                .value_parser(clap::value_parser!(PathBuf))
                .value_hint(ValueHint::FilePath)
                .required(false)
                .default_value(".env"),
        );

    let args = cmd.get_matches();
    dotenv::from_filename(
        args.get_one::<PathBuf>("config")
            .expect("config file is bad path?"),
    )
    .ok();

    let register_globally = env::var("ICSSC_REGISTER_GLOBAL").is_ok();
    let register_locally = env::var("ICSSC_REGISTER_LOCAL").is_ok();
    let guilds_to_register_in = env::var("ICSSC_GUILDS")
        .unwrap_or(String::from(""))
        .split(",")
        .map(String::from)
        .map(|s| String::from(s.trim()))
        .filter(|s| !s.is_empty())
        .map(|id| GuildId::from(id.parse::<u64>().expect("guild id not valid snowflake")))
        .collect_vec();

    let db_url = env::var("DATABASE_URL").expect("need postgres URL!");

    let framework = poise::Framework::<BotVars, BotError>::builder()
        .options(FrameworkOptions {
            commands: vec![meta::ping(), commands::snipe::snipe()],
            prefix_options: PrefixFrameworkOptions {
                mention_as_prefix: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let no_commands = &[] as &[Command<(), BotError>];

                let commands_count =
                    pluralize("command", framework.options().commands.len() as isize, true);

                if register_globally {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    for id in guilds_to_register_in.iter() {
                        poise::builtins::register_in_guild(ctx, no_commands, *id).await?;
                    }
                    println!("registered {commands_count} globally");
                } else {
                    println!("not registering {commands_count} globally");
                }

                if register_locally {
                    poise::builtins::register_globally(ctx, no_commands).await?;

                    for id in guilds_to_register_in.iter() {
                        poise::builtins::register_in_guild(ctx, &framework.options().commands, *id)
                            .await?;
                    }
                    println!(
                        "registered {commands_count} locally in {}",
                        pluralize("guild", guilds_to_register_in.len() as isize, true)
                    );
                }

                let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url);
                let db_pool = Pool::builder(config).build()?;

                Ok(BotVars { db_pool })
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
    .event_handler(handler::ICSSpottingsCouncilEventHandler)
    .framework(framework)
    .await
    .expect("couldn't make client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

type BotError = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, BotVars, BotError>;
