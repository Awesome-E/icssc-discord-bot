use crate::util::paginate::{EmbedLinePaginator, PaginatorOptions};
use crate::{BotError, Context};
use anyhow::Context as _;
use entity::user_stat;
use itertools::Itertools;
use migration::NullOrdering;
use poise::ChoiceParameter;
use sea_orm::sea_query::Expr;
use sea_orm::{EntityTrait, FromQueryResult, Order, QueryOrder, QuerySelect};
use serenity::all::{Mentionable, UserId};
use std::num::NonZeroUsize;

#[derive(ChoiceParameter, PartialEq, Eq, Copy, Clone, Debug, Hash, Default)]
enum LeaderboardBy {
    #[default]
    #[name = "Total snipes"]
    SnipeCount,
    #[name = "Times sniped"]
    VictimCount,
    #[name = "Ratio of total snipes to times sniped"]
    SnipeRate,
}

#[derive(FromQueryResult)]
struct SnipeRateQuery {
    id: i64,
    snipe_rate: Option<f64>,
}

/// Show leaderboards by various sniping statistics
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Leaderboard type; default is \"Total snipes\'"] by: Option<LeaderboardBy>,
) -> Result<(), BotError> {
    let by = by.unwrap_or_default();

    let lines = match by {
        LeaderboardBy::SnipeCount => user_stat::Entity::find()
            .order_by_desc(user_stat::Column::SnipesInitiated)
            .all(&ctx.data().db)
            .await
            .context("fetch leaderboard from db")?
            .into_iter()
            .enumerate()
            .map(|(i, mdl)| {
                format!(
                    "{}. {}: {}",
                    i + 1,
                    UserId::from(mdl.id as u64).mention(),
                    mdl.snipes_initiated
                )
                .into_boxed_str()
            })
            .collect_vec(),
        LeaderboardBy::VictimCount => user_stat::Entity::find()
            .order_by_desc(user_stat::Column::SnipesVictim)
            .all(&ctx.data().db)
            .await
            .context("fetch leaderboard from db")?
            .into_iter()
            .enumerate()
            .map(|(i, mdl)| {
                format!(
                    "{}. {}: {}",
                    i + 1,
                    UserId::from(mdl.id as u64).mention(),
                    mdl.snipes_victim
                )
                .into_boxed_str()
            })
            .collect_vec(),
        LeaderboardBy::SnipeRate => user_stat::Entity::find()
            .select_only()
            .column(user_stat::Column::Id)
            .column_as(
                Expr::col(user_stat::Column::SnipesInitiated)
                    .div(Expr::col(user_stat::Column::SnipesVictim)),
                "snipe_rate",
            )
            .order_by_with_nulls(Expr::col("snipe_rate"), Order::Desc, NullOrdering::Last)
            .order_by_desc(user_stat::Column::SnipesInitiated)
            .into_model::<SnipeRateQuery>()
            .all(&ctx.data().db)
            .await
            .context("fetch leaderboard from db")?
            .into_iter()
            .enumerate()
            .map(|(i, mdl)| {
                format!(
                    "{}. {}: {}",
                    i + 1,
                    UserId::from(mdl.id as u64).mention(),
                    mdl.snipe_rate
                        .map(|n| n.to_string())
                        .unwrap_or(String::from("N/A"))
                )
                .into_boxed_str()
            })
            .collect_vec(),
    };

    let paginator = EmbedLinePaginator::new(
        lines,
        PaginatorOptions::default()
            .sep("\n")
            .max_lines(NonZeroUsize::new(10).unwrap())
            .ephemeral(true),
    );

    paginator.run(ctx).await.context("start paginator")?;
    Ok(())
}
