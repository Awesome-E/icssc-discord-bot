use crate::{BotError, Context};

/// Check bot is alive, get numerical ping to Discord
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn ping(ctx: Context<'_>) -> Result<(), BotError> {
    let ping_num = ctx.ping().await.as_millis();

    ctx.say(match ping_num {
        0 => String::from("ok, waiting for more data to report ping"),
        _ => format!("hi, heartbeat is pinging in {} ms", ping_num),
    })
    .await?;
    Ok(())
}
