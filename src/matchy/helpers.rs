use crate::{AppError, Context};
use anyhow::Context as _;
use entity::{matchy_meetup_pair, matchy_meetup_pair_member, matchy_meetup_round};
use itertools::Itertools;
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
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

/// Hashes a string into a u64 that can be used as a seed
pub fn hash_seed(seed: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    hasher.finish()
}

/// Generates a short checksum for a given seed & pairing, which can be used to verify that nothing
/// has changed between multiple uses.
pub fn checksum_matching<T: Hash>(seed: u64, pairs: &[Match<T>]) -> String {
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
pub fn format_pairs(pairs: &[Match<UserId>]) -> String {
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

pub(crate) async fn add_pairings_to_db(
    ctx: &Context<'_>,
    pairs: Vec<Vec<UserId>>,
) -> Result<(), AppError> {
    let round_sql = matchy_meetup_round::ActiveModel {
        id: Default::default(),
        created_at: Default::default(),
    };

    let conn = &ctx.data().db;
    conn.transaction::<_, (), AppError>(move |txn| {
        Box::pin(async move {
            let round = round_sql.insert(txn).await.context("insert round")?;

            for pair in pairs {
                let pair_sql = matchy_meetup_pair::ActiveModel {
                    id: Default::default(),
                    round_id: Set(round.id),
                };
                let pair_sql = pair_sql.insert(txn).await.context("insert pair")?;

                for user_id in pair {
                    let pair_member_sql = matchy_meetup_pair_member::ActiveModel {
                        pair_id: Set(pair_sql.id),
                        discord_uid: Set(user_id.into()),
                    };
                    pair_member_sql
                        .insert(txn)
                        .await
                        .context("insert pair member")?;
                }
            }
            Ok(())
        })
    })
    .await
    .context("pairing insert fail")
}
