use crate::{AppError, Context};
use crate::spottings::{
    leaderboard::leaderboard,
    snipe::history,
    snipe::post,
    privacy::{check_snipes_participation, set_snipes_participation}
};


#[poise::command(prefix_command, slash_command, subcommands("leaderboard", "post", "history", "check_snipes_participation", "set_snipes_participation"))]
pub(crate) async fn spottings(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}
