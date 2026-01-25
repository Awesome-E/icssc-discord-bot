use super::discord_helpers::match_members;
use super::helpers::Pairing;
use super::helpers::{format_id, format_pairs, hash_seed};
use crate::Context;
use anyhow::Result;
use itertools::Itertools as _;

async fn handle_create_pairing(ctx: Context<'_>, seed_str: String) -> Result<String> {
    let seed = hash_seed(&seed_str);

    let Pairing(pairs, imperfect_matches) = match_members(ctx, seed).await?;
    let pairs_str = format_pairs(&pairs);
    let key = format!(
        "{}_{}",
        seed_str,
        super::helpers::checksum_matching(seed, &pairs)
    );
    let num_members: usize = pairs.iter().map(Vec::len).sum();
    let imperfect_matches_message = if imperfect_matches.is_empty() {
        "All members were matched with new people".to_owned()
    } else {
        format!(
            "The following members could only be matched with people they may have matched with before: {}",
            imperfect_matches.iter().map(format_id).join(", ")
        )
    };
    Ok(format!(
        "{pairs_str}\nTotal paired members: {num_members}\n{imperfect_matches_message}\nTo send this pairing, use this key: `{key}`"
    ))
}

/// Generate a potential pairing of users who have opted in to Matchy Meetups
#[poise::command(
    slash_command,
    hide_in_help,
    ephemeral,
    rename = "create",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn create_pairing(
    ctx: Context<'_>,
    #[description = "A seed to use for the generated pairing (for example, use the current date)."]
    seed: String,
) -> Result<()> {
    ctx.defer_ephemeral().await?;
    let resp = handle_create_pairing(ctx, seed)
        .await
        .unwrap_or_else(|e| format!("Error: {e}"));
    ctx.say(resp).await?;
    Ok(())
}
