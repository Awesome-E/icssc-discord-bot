use std::collections::{HashMap, HashSet};

use crate::{
    AppError, Context,
    util::{
        ContextExtras as _,
        gdrive::{DriveFilePermissionRole, get_file_permissions},
        roster::{RosterSheetRow, get_bulk_members_from_roster},
    },
};
use anyhow::Context as _;
use itertools::Itertools as _;
use serenity::{all::Mentionable as _, futures::StreamExt as _};

/// Get a list of server members whose roles are out of sync with the roster
#[poise::command(slash_command, hide_in_help, ephemeral)]
pub(crate) async fn check_discord_roles(ctx: Context<'_>) -> Result<(), AppError> {
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

// in case we add more emails, e.g. club advisor, later
fn is_admin_email(email: &str) -> bool {
    email == "icssc@uci.edu"
}

/// Check whether Google Drive access is desynced from the roster
#[poise::command(slash_command, hide_in_help, ephemeral)]
pub(crate) async fn check_google_access(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.defer_ephemeral().await?;
    let data = ctx.data();

    let roster = get_bulk_members_from_roster(data, &[]).await?;
    let roster_lookup = roster
        .iter()
        .map(|row| (&row.email, row))
        .collect::<HashMap<&String, &RosterSheetRow>>();

    let drive_permissions = get_file_permissions(data)
        .await
        .context("Failed to fetch permissions; ensure service account has access")?;

    drive_permissions
        .iter()
        .find(|u| {
            u.email_address == "icssc@uci.edu"
                && matches!(u.role, DriveFilePermissionRole::Organizer)
        })
        .context("expected icssc@uci.edu to have organizer access")?;

    let board = &String::from("board");
    let mut desynced = Vec::new();

    // ensure no one on the roster is missing from the drive_permissions list
    // insufficient permissions are handled when iterating the drive_permissions list, not here
    let mut roster_iter = roster.iter();
    let emails_with_access = drive_permissions
        .iter()
        .map(|u| u.email_address.as_str())
        .collect::<HashSet<&str>>();

    while let Some(roster_user) = roster_iter.next()
        && desynced.len() < 20
    {
        let expected = match roster_user.committees.contains(board) {
            true => "`Manager`",
            false => "`Editor` or `Content Manager`",
        };
        if !emails_with_access.contains(roster_user.email.as_str()) {
            desynced.push(format!(
                "1. Missing: `{}` is not {}",
                &roster_user.email, expected
            ));
        }
    }

    // ensure that all drive_permissions are found in the roster and are consistent with committee.
    let mut perms_iter = drive_permissions.iter();
    while let Some(google_user) = perms_iter.next()
        && desynced.len() < 20
    {
        let email = &google_user.email_address;
        if *email == data.env.service_account_key.email {
            continue;
        }

        let error = match roster_lookup.get(email) {
            Some(val) => match (val.committees.contains(board), &google_user.role) {
                (true, DriveFilePermissionRole::Organizer)
                | (
                    false,
                    DriveFilePermissionRole::FileOrganizer | DriveFilePermissionRole::Writer,
                ) => None,
                (true, _) => Some(format!("1. Insufficient: `{email}` is not `Manager`")),
                (false, DriveFilePermissionRole::Organizer) => Some(format!(
                    "1. Unexpected: `{email}` should be `Editor` or `Content Manager`, not `Manager`",
                )),
                (false, _) => Some(format!(
                    "1. Insufficient: `{email}` is not `Editor` or `Content Manager`",
                )),
            },
            None if is_admin_email(email) => match &google_user.role {
                DriveFilePermissionRole::Organizer => None,
                _ => Some(format!("1. Insufficient: `{email}` is not `Manager`")),
            },
            None => Some(format!("1. Unexpected: `{email}`")),
        };

        if let Some(why) = error {
            desynced.push(why);
        }
    }

    let text = match desynced.len() {
        0 => String::from("All Google Drive users are in sync!"),
        _ => desynced.join("\n"),
    };

    ctx.reply_ephemeral(text).await?;

    Ok(())
}
