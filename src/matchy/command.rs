use crate::{
    AppError, Context,
    matchy::{
        create_pairing::create_pairing, dump_pairings::dump_pairings, send_pairing::send_pairing,
    },
};

#[poise::command(
    prefix_command,
    slash_command,
    subcommands("create_pairing", "dump_pairings", "send_pairing")
)]
pub(crate) async fn matchy(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}
