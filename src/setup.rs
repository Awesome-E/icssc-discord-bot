use clap::ValueHint;
use itertools::Itertools;
use poise::{BoxFuture, Command, Framework};
use pluralizer::pluralize;
use serenity::all::{Context,Ready,GuildId};
use std::path::PathBuf;
use std::env;

use crate::{BotError, BotVars};

pub(crate) fn load_env() -> () {
    let cmd = clap::command!("icssc-discord-bot")
        .about("The somewhat official Discord bot for ICS Student Council")
        .arg(
            clap::arg!(["config"] ".env file path")
                .value_parser(clap::value_parser!(PathBuf))
                .value_hint(ValueHint::FilePath)
                .default_value(".env"),
        );

    let args = cmd.get_matches();
    dotenv::from_filename(
        args.get_one::<PathBuf>("config")
            .expect("config file is bad path?"),
    ).ok();
}

async fn register_commands (ctx: &Context, framework: &Framework<BotVars, anyhow::Error>) -> Result<(), BotError> {
    let is_global = env::var("ICSSC_REGISTER_GLOBAL").is_ok();
    let no_commands = &[] as &[Command<BotVars, BotError>];
    let commands = &framework.options().commands;
    let global_registration = if is_global { commands } else { no_commands };
    let local_registration = if is_global { no_commands } else { commands };
    let guilds = env::var("ICSSC_GUILDS")
        .unwrap_or(String::from(""))
        .split(",")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|id| GuildId::from(id.parse::<u64>().expect("guild id not valid snowflake")))
        .collect_vec();

    poise::builtins::register_globally(ctx, global_registration).await?;

    for id in guilds.iter() {
        poise::builtins::register_in_guild(ctx, local_registration, *id).await?;
    }

    let commands_text = pluralize("command", framework.options().commands.len() as isize, true);
    if is_global {
        println!("[setup] Registered {commands_text} globally");
    } else {
        let guilds_text = pluralize("guild", guilds.len() as isize, true);
        println!("[setup] Registered {commands_text} locally in {guilds_text}");
    }

    Ok(())
}

pub(crate) fn framework_setup<'a>(
    ctx: &'a Context,
    _ready: &'a Ready,
    framework: &'a Framework<BotVars, BotError>
) -> BoxFuture<'a, Result<BotVars, BotError>> {
    Box::pin(async move {
        register_commands(&ctx, &framework).await?;

        let db_url = env::var("DATABASE_URL").expect("need postgres URL!");
        let db = sea_orm::Database::connect(&db_url).await.unwrap();

        Ok(BotVars { db })
    })
}
