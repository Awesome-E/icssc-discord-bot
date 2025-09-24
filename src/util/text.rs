use discord_md::generate::{ToMarkdownString, ToMarkdownStringOption};
use itertools::Itertools;
use serenity::all::{Permissions, UserId};
use std::fmt::Display;

pub(crate) fn bot_invite_url(
    id: UserId,
    permissions: Permissions,
    with_slash_commands: bool,
) -> String {
    let perms_section = permissions.bits().to_string();
    format!(
        "https://discord.com/oauth2/authorize?client_id={id}&permissions={perms_section}&integration_type=0&scope=bot{}",
        if with_slash_commands {
            "+applications.commands"
        } else {
            ""
        }
    )
}

pub(crate) fn remove_markdown(input: &str) -> String {
    let doc = discord_md::parse(input);

    doc.to_markdown_string(&ToMarkdownStringOption::new().omit_format(true))
}

pub(crate) fn comma_join(mut items: impl ExactSizeIterator<Item = impl Display>) -> String {
    match items.len() {
        0 => String::from(""),
        1 => items.next().unwrap().to_string(),
        2 => format!("{} and {}", items.next().unwrap(), items.next().unwrap()),
        _ => {
            let all = items.map(|it| it.to_string()).collect_vec();
            format!(
                "{}, and {}",
                all[..all.len() - 1].join(", "),
                all[all.len() - 1]
            )
        }
    }
}
