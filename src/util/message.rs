use std::collections::HashSet;

use serenity::all::Message;

pub(crate) fn get_members(message: &Message, include_author: bool) -> HashSet<String> {
    let mut members: HashSet<String> = message
        .mentions
        .iter()
        .map(|user| user.id.to_string())
        .collect();

    if include_author {
        members.insert(message.author.id.to_string());
    }
    members
}
