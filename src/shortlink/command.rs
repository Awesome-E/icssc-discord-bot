// subcommands: create, check

use crate::{
    AppError, AppContext,
    shortlink::{check::check, create::create},
};

#[poise::command(
    prefix_command,
    slash_command,
    subcommands("check", "create"),
    guild_only
)]
pub(crate) async fn shortlink(ctx: AppContext<'_>) -> Result<(), AppError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}
