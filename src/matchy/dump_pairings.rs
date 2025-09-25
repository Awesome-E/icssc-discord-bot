use crate::matchy::participation::get_previous_matches;
use crate::Context;
use anyhow::{Error, Result};
use crate::matchy::helpers::add_pairings_to_db;

async fn handle_dump_pairings(ctx: &Context<'_>) -> Result<String> {
    let prev_matches = get_previous_matches(ctx.data()).await?;

    add_pairings_to_db(ctx, prev_matches).await?;

    Ok(String::from("Dumped pairings to database"))
}

/// Dump pairing history from the current into the database
#[poise::command(slash_command, hide_in_help, required_permissions = "ADMINISTRATOR")]
pub async fn dump_pairings(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let resp = handle_dump_pairings(&ctx)
        .await
        .unwrap_or_else(|e| format!("Error: {e}"));
    println!("{resp}");
    ctx.say(resp).await?;
    Ok(())
}
