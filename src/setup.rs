use crate::util::ContextExtras;
use crate::{AppError, AppVars, AppVarsInner};
use crate::{attendance, internal_commands, matchy, spottings};
use clap::ArgMatches;
use itertools::Itertools;
use pluralizer::pluralize;
use poise::{BoxFuture, Command, Framework, FrameworkError, FrameworkOptions};
use serenity::FutureExt;
use serenity::all::{Context, GuildId};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn load_env(args: &ArgMatches) {
    dotenv::from_filename(
        args.get_one::<PathBuf>("config")
            .expect("config file is bad path?"),
    )
    .ok();
}

pub(crate) async fn register_commands(
    data: Arc<AppVarsInner>,
    ctx: &Context,
    framework: &Framework<AppVars, AppError>,
) -> Result<(), AppError> {
    let is_global = data.env.bot.commands.register_globally != "";
    let no_commands = &[] as &[Command<AppVars, AppError>];
    let commands = &framework.options().commands;
    let global_registration = if is_global { commands } else { no_commands };
    let local_registration = if is_global { no_commands } else { commands };
    let guilds = data.env.bot.commands.guilds
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

fn handle_framework_error(error: FrameworkError<AppVars, AppError>) -> BoxFuture<()> {
    async move {
        println!("Error: {error}");

        let Some(ctx) = error.ctx() else { return };
        let error_res = match error {
            FrameworkError::Command {
                error: wrapped_error,
                ..
            } => {
                ctx.reply_ephemeral(format!("An unexpected error occurred: {wrapped_error:?}"))
                    .await
            }
            _ => ctx.reply_ephemeral("An unknown error occurred").await,
        };
        if let Err(e) = error_res {
            println!("A further error occurred sending the error message to discord: {e:?}")
        }
    }
    .boxed()
}

// fn check_command_invocation(
//     ctx: poise::Context<AppVars, AppError>,
// ) -> BoxFuture<Result<bool, AppError>> {
//     const ICSSC_SERVER: u64 = 760915616793755669;
//     const ALLOWED_CHANNELS: &[u64] = &[1328907402321592391, 1338632123929591970];
//
//     async move {
//         Ok(ctx.guild_id() != Some(GuildId::from(ICSSC_SERVER))
//             || ALLOWED_CHANNELS.contains(&ctx.channel_id().into()))
//     }
//     .boxed()
// }

fn get_bot_commands() -> Vec<Command<AppVars, AppError>> {
    vec![
        attendance::checkin::attended(),
        attendance::checkin::checkin(),
        matchy::create_pairing::create_pairing(),
        matchy::send_pairing::send_pairing(),
        matchy::dump_pairings::dump_pairings(),
        spottings::meta::ping(),
        spottings::snipe::spotting(),
        spottings::snipe::log_message_snipe(),
        spottings::leaderboard::leaderboard(),
        spottings::privacy::opt_out(),
        internal_commands::calendar::calendar_command(),
    ]
}

pub(crate) fn create_bot_framework_options() -> FrameworkOptions<AppVars, AppError> {
    FrameworkOptions {
        on_error: handle_framework_error,
        commands: get_bot_commands(),
        // command_check: Some(check_command_invocation),
        ..Default::default()
    }
}
