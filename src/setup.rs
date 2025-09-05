use itertools::Itertools;
use poise::{BoxFuture,Command,Framework,FrameworkError, FrameworkOptions};
use pluralizer::pluralize;
use serenity::all::{Context,Ready,GuildId};
use std::env;
use crate::util::ContextExtras;
use crate::matchy;
use crate::spottings;
use serenity::{FutureExt};
use crate::{BotError, BotVars};

async fn register_commands(ctx: &Context, framework: &Framework<BotVars, anyhow::Error>) -> Result<(), BotError> {
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
    async move {
        register_commands(&ctx, &framework).await?;

        let db_url = env::var("DATABASE_URL").expect("need postgres URL!");
        let db = sea_orm::Database::connect(&db_url).await.unwrap();

        Ok(BotVars { db })
    }.boxed()
}

fn handle_framework_error(error: FrameworkError<BotVars, anyhow::Error>) -> BoxFuture<()> {
    Box::pin(async move {
        println!("Error: {}", error);

        let Some(ctx) = error.ctx() else { return };
        let error_res = match error {
            FrameworkError::Command { error: wrapped_error, .. } => {
                ctx.reply_ephemeral(format!(
                    "An unexpected error occurred: {:?}",
                    wrapped_error
                ))
                .await
            }
            _ => ctx.reply_ephemeral("An unknown error occurred").await,
        };
        if let Err(e) = error_res {
            println!("A further error occurred sending the error message to discord: {:?}", e)
        }
    })
}

fn check_command_invocation<'a>(ctx: poise::Context<'a, BotVars, anyhow::Error>) -> BoxFuture<'a, Result<bool, anyhow::Error>> {
    const ICSSC_SERVER: u64 = 760915616793755669;
    const ALLOWED_CHANNELS: &[u64] = &[1328907402321592391, 1338632123929591970];

    async move {
        Ok(ctx.guild_id() != Some(GuildId::from(ICSSC_SERVER))
            || ALLOWED_CHANNELS.contains(&ctx.channel_id().into()))
    }
    .boxed()
}

fn get_bot_commands() -> Vec<Command<BotVars, anyhow::Error>> {
    return vec![
        matchy::create_pairing::create_pairing(),
        matchy::send_pairing::send_pairing(),
        spottings::meta::ping(),
        spottings::snipe::snipe(),
        spottings::leaderboard::leaderboard(),
        spottings::privacy::opt_out(),
    ];
}

pub(crate) fn create_bot_framework_options() -> FrameworkOptions<BotVars, anyhow::Error> {
    FrameworkOptions {
        on_error: handle_framework_error,
        commands: get_bot_commands(),
        command_check: Some(check_command_invocation),
        ..Default::default()
    }
}
