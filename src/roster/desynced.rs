use std::collections::{HashMap, HashSet};

use crate::{
    AppError, Context,
    util::{
        ContextExtras as _,
        roster::{RosterSheetRow, get_bulk_members_from_roster},
    },
};
use anyhow::Context as _;
use itertools::Itertools as _;
use serenity::{all::Mentionable as _, futures::StreamExt as _};

/// Get a list of members that are out of sync with the roster
#[poise::command(slash_command, hide_in_help, ephemeral)]
pub(crate) async fn desynced(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.defer_ephemeral().await?;

    let roster = get_bulk_members_from_roster(ctx.data(), &[]).await?;
    let roster_lookup = roster
        .iter()
        .map(|row| (&row.discord, row))
        .collect::<HashMap<&String, &RosterSheetRow>>();

    let guild = ctx.guild_id().context("get guild id")?;
    let role_map = guild.roles(ctx.http()).await?;
    let committee_names = roster
        .iter()
        .flat_map(|member| &member.committees)
        .unique()
        .collect::<HashSet<&String>>();

    let mut member_iter = guild.members_iter(&ctx).boxed();
    let mut desynced = Vec::new();

    while let Some(guild_member) = member_iter.next().await
        && desynced.len() < 10
    {
        let guild_member = guild_member?;
        let roster_committees = match roster_lookup.get(&guild_member.user.name) {
            Some(roster_member) => &roster_member.committees,
            None => &vec![],
        };

        let gm_role_names = guild_member
            .roles
            .iter()
            .filter_map(|role| Some(&role_map.get(role)?.name))
            .collect_vec();

        let extra_roles = gm_role_names
            .iter()
            .filter(|&name| committee_names.contains(name) && !roster_committees.contains(name))
            .collect_vec();

        let missing_roles = roster_committees
            .iter()
            .filter(|name| !gm_role_names.contains(name))
            .collect_vec();

        if extra_roles.is_empty() && missing_roles.is_empty() {
            continue;
        }

        desynced.push(format!(
            "1. {} desynced!\n  - Missing: {}\n  - Unexpected: {}",
            guild_member.mention(),
            missing_roles.iter().join(", "),
            extra_roles.iter().join(", "),
        ));
    }

    let text = match desynced.len() {
        0 => String::from("All users are in sync!"),
        _ => desynced.join("\n"),
    };

    ctx.reply_ephemeral(text).await?;

    Ok(())
}
