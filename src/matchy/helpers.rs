use super::types::Data;
use anyhow::Error;
use itertools::Itertools;
use poise::FrameworkError;
use serenity::all::UserId;
use std::hash::{DefaultHasher, Hash, Hasher};

/// A Match represents a single set of elements matched together. In the context of matchy meetups
/// most Matches are pairs, but if there are an odd number there will be one 3-matching.
pub type Match<T> = Vec<T>;

/// A pairing contains the matchings for some group of elements.
/// The first element contains the matchings (each element will appear in exactly one Match)
/// The second element contains the set of duplicated matchings, if any. These are the elements
/// that were unable to be matched with unique elements. Each element in this second vector
/// also appears somewhere in the first set of matchings.
pub struct Pairing<T>(pub Vec<Match<T>>, pub Vec<T>);

/// Logs an error to stdout.
pub async fn handle_error(error: poise::FrameworkError<'_, Data, Error>) {
    println!("Error: {:?}", error);

    let Some(ctx) = error.ctx() else { return };
    let error_res = match error {
        FrameworkError::Command {
            error: wrapped_error,
            ..
        } => {
            ctx.say(format!("An unexpected error occurred: {:?}", wrapped_error))
                .await
        }
        _ => ctx.say("An unknown error occurred").await,
    };
    if let Err(e) = error_res {
        println!(
            "A further error occurred sending the error message to discord: {:?}",
            e
        )
    }
}

/// Hashes a string into a u64 that can be used as a seed
pub fn hash_seed(seed: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    hasher.finish()
}

/// Generates a short checksum for a given seed & pairing, which can be used to verify that nothing
/// has changed between multiple uses.
pub fn checksum_matching<T: Hash>(seed: u64, pairs: &Vec<Match<T>>) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    pairs.hash(&mut hasher);
    let hex = format!("{:x}", hasher.finish());
    hex[..8].to_string()
}

/// Formats an ID for display as a ping in discord
pub fn format_id(id: &UserId) -> String {
    format!("<@{id}>")
}

/// Formats a pairing into a string suitable for a discord message
pub fn format_pairs(pairs: &Vec<Match<UserId>>) -> String {
    pairs
        .iter()
        .map(|p| {
            p.iter().take(p.len() - 1).map(format_id).join(", ")
                + if p.len() > 2 { ", and " } else { " and " }
                + &format_id(p.last().expect("pairings should be non-empty"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
