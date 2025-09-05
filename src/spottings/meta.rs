use crate::{BotError, Context};
use crate::util::ContextExtras;

async fn check_db_ok(ctx: &Context<'_>) -> Result<(), BotError> {
    ctx.data().db.ping().await?;
    Ok(())
}

/// Check bot is alive, get numerical ping to Discord
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn ping(ctx: Context<'_>) -> Result<(), BotError> {
    let ping_num = ctx.ping().await.as_millis();

    ctx.reply_ephemeral(format!(
        "{}\n\n{}",
        match ping_num {
            0 => String::from("ok, waiting for more data to report ping"),
            _ => format!("hi, heartbeat is pinging in {} ms", ping_num),
        },
        match check_db_ok(&ctx).await {
            Ok(_) => String::from("postgres ok"),
            Err(err) => format!("postgres not ok: {}", err),
        }
    ))
    .await?;
    Ok(())
}
