use crate::schema::user_stat;
use crate::util::paginate::{EmbedLinePaginator, PaginatorOptions};
use crate::{BotError, Context};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use itertools::Itertools;
use poise::ChoiceParameter;
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

/// Show leaderboards by various sniping statistics
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Leaderboard type; default is \"Total snipes\'"] by: Option<LeaderboardBy>,
) -> Result<(), BotError> {
    let mut conn = ctx.data().db_pool.get().await?;

    let by = by.unwrap_or_default();

    let lines = match by {
        LeaderboardBy::SnipeCount => user_stat::table
            .select((user_stat::id, user_stat::snipe))
            .order_by(user_stat::snipe.desc())
            .load::<(i64, i64)>(&mut conn)
            .await?
            .into_iter()
            .enumerate()
            .map(|(i, (u_id, n))| {
                format!("{}. {}: {}", i + 1, UserId::from(u_id as u64).mention(), n)
                    .into_boxed_str()
            })
            .collect_vec(),
        LeaderboardBy::VictimCount => user_stat::table
            .select((user_stat::id, user_stat::sniped))
            .order_by(user_stat::sniped.desc())
            .load::<(i64, i64)>(&mut conn)
            .await?
            .into_iter()
            .enumerate()
            .map(|(i, (u_id, n))| {
                format!("{}. {}: {}", i + 1, UserId::from(u_id as u64).mention(), n)
                    .into_boxed_str()
            })
            .collect_vec(),
        LeaderboardBy::SnipeRate => user_stat::table
            .select((user_stat::id, user_stat::snipe_rate))
            .order_by(user_stat::snipe_rate.desc())
            .load::<(i64, Option<f64>)>(&mut conn)
            .await?
            .into_iter()
            .enumerate()
            .map(|(i, (u_id, n))| {
                format!(
                    "{}. {}: {}",
                    i + 1,
                    UserId::from(u_id as u64).mention(),
                    n.map(|n| n.to_string()).unwrap_or(String::from("N/A"))
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

    paginator.run(ctx).await?;
    Ok(())
}
