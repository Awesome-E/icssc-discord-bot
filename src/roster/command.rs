use crate::{
    AppError, AppContext,
    roster::desynced::{check_discord_roles, check_google_access},
};

#[poise::command(
    prefix_command,
    slash_command,
    subcommands("check_discord_roles", "check_google_access"),
    guild_only
)]
pub(crate) async fn roster(ctx: AppContext<'_>) -> Result<(), AppError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}
