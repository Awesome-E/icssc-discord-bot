use crate::{AppError, Context, roster::desynced::desynced};

#[poise::command(prefix_command, slash_command, subcommands("desynced"), guild_only)]
pub(crate) async fn roster(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}
