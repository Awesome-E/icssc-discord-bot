use anyhow::{Context, Result, bail, ensure};
use itertools::Itertools;
use sea_orm::{EntityTrait, FromQueryResult, QuerySelect, RelationTrait};
use sea_orm::sea_query::Expr;
use serenity::all::UserId;
use entity::{matchy_meetup_opt_in, matchy_meetup_pair_member};
use crate::BotVars;


/// Gets the currently opted in participants for Matchy Meetups
pub(crate) async fn get_current_opted_in(data: &BotVars) -> Result<Vec<UserId>> {
    let opted_in = matchy_meetup_opt_in::Entity::find()
        .all(&data.db)
        .await
        .context("fetch opt in from db")?
        .into_iter()
        .map(|row| UserId::from(row.user_id as u64))
        .collect_vec();

    Ok(opted_in)
}

#[derive(FromQueryResult)]
struct GroupedPairMembers {
    members: Vec<i64>
}

/// Fetching pairs from previous matchy meetups
pub(crate) async fn get_previous_matches(data: &BotVars) -> Result<Vec<Vec<UserId>>> {
    let matches = matchy_meetup_pair_member::Entity::find()
        .select_only()
        .column_as(Expr::cust(r#"ARRAY_AGG(matchy_meetup_pair_member.discord_uid)"#), "members")
        .group_by(matchy_meetup_pair_member::Column::PairId)
        .into_model::<GroupedPairMembers>()
        .all(&data.db)
        .await
        .context("fetch history from db")?
        .into_iter()
        .map(|row| {
            row.members.into_iter().map(|uid| UserId::from(uid as u64)).collect_vec()
        })
        .collect_vec();

    Ok(matches)
}

