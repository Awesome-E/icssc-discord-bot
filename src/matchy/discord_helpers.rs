use super::ROLE_NAME;
use super::config::HISTORY_CHANNEL_NAME;
use super::helpers::{Match, Pairing};
use super::matching::graph_pair;
use crate::Context;
use anyhow::{Context as _, Result, bail};
use chrono::{Duration, Local};
use itertools::Itertools;
use poise::futures_util::StreamExt;
use regex::Regex;
use serenity::all::{ChannelId, GuildChannel, GuildId, PartialGuild, RoleId, UserId};

pub async fn find_channel(
    ctx: &Context<'_>,
    guild_id: GuildId,
    name: &str,
) -> Result<Option<GuildChannel>> {
    Ok(guild_id
        .channels(ctx)
        .await?
        .into_iter()
        .find(|(_, c)| c.name == name)
        .map(|(_, c)| c))
}

/// Returns a vector of all guild members with the specified role ID.
async fn guild_members_with_role(
    ctx: &Context<'_>,
    guild: &PartialGuild,
    role_id: RoleId,
) -> Result<Vec<UserId>> {
    // max number of pages to try to fetch (to avoid infinite loops in the event of the server
    // response format changing in a way that breaks the end-of-page detection)
    const MAX_PAGES: u64 = 20;

    // maximum number of members to request per page
    const PAGE_LIMIT: u64 = 1000;

    let mut last_member = None;
    let mut members_with_role = Vec::new();

    for _ in 0..MAX_PAGES {
        let page = guild.members(&ctx, Some(PAGE_LIMIT), last_member).await?;

        members_with_role.extend(
            page.iter()
                .filter(move |u| u.roles.iter().contains(&role_id))
                .map(|p| p.user.id),
        );

        if page.len() < PAGE_LIMIT as usize {
            break;
        }
        last_member = Some(
            page.last()
                .expect("page is never empty here if PAGE_LIMIT > 0")
                .user
                .id,
        );
    }

    Ok(members_with_role)
}

/// Returns an iterable over all previous pairings
pub async fn previous_matches(
    ctx: &Context<'_>,
    channel_id: ChannelId,
) -> anyhow::Result<Vec<Match<UserId>>> {
    const MAX_MESSAGES_TO_REQUEST: usize = 1000;

    let mut pairings: Vec<Vec<UserId>> = Vec::new();

    let re = Regex::new(r"<@([0-9]+)>").expect("regex creation should succeed");

    let mut messages = channel_id
        .messages_iter(&ctx)
        .boxed()
        .take(MAX_MESSAGES_TO_REQUEST);

    while let Some(message_result) = messages.next().await {
        match message_result {
            Ok(message) => {
                if *message.timestamp < Local::now() - Duration::days(365) {
                    // this message was not within the last year
                    break;
                }
                pairings.extend(
                    message
                        .content
                        .split("\n")
                        .map(|line| {
                            re.captures_iter(line)
                                .map(|c| c.extract())
                                .flat_map(|(_, [id])| id.parse().ok())
                                .collect()
                        })
                        .filter(|pair: &Vec<_>| pair.len() > 1),
                );
            }
            Err(error) => bail!("Error fetching message history: {}", error),
        }
    }
    Ok(pairings)
}

/// Pairs members with ROLE_NAME in the guild together.
/// The result is a pairing of
pub async fn match_members(ctx: Context<'_>, seed: u64) -> Result<Pairing<UserId>> {
    let guild = ctx
        .partial_guild()
        .await
        .context("This command must be called from a guild (server).")?
        .clone();
    let Some(role) = guild.role_by_name(ROLE_NAME) else {
        bail!("Could not find a role with name `{ROLE_NAME}`");
    };
    let Some(history_channel) = find_channel(&ctx, guild.id, HISTORY_CHANNEL_NAME).await? else {
        bail!("Could not find history channel");
    };
    let participants = guild_members_with_role(&ctx, &guild, role.id).await?;
    if participants.len() <= 1 {
        bail!(
            "Need at least two members to create a pairing (found {} member{} with role <@&{}>).",
            participants.len(),
            if participants.len() == 1 { "" } else { "s" },
            role.id
        );
    }
    Ok(graph_pair(
        participants,
        &previous_matches(&ctx, history_channel.id).await?,
        seed,
    )?)
}
