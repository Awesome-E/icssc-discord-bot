use crate::{BotError, Context};
use diesel::Connection;

/// Check bot is alive, get numerical ping to Discord
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn ping(ctx: Context<'_>) -> Result<(), BotError> {
    let ping_num = ctx.ping().await.as_millis();

    let conn = ctx.data().db_conn();

    ctx.say(format!(
        "{}\n\n{}",
        match ping_num {
            0 => String::from("ok, waiting for more data to report ping"),
            _ => format!("hi, heartbeat is pinging in {} ms", ping_num),
        },
        match conn.map(|mut conn| conn.begin_test_transaction()) {
            Ok(_) => String::from("postgres ok"),
            Err(err) => format!("postgres not ok: {}", err),
        }
    ))
    .await?;
    Ok(())
}
