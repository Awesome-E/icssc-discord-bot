use crate::schema::{message, snipe};
use crate::util::paginate::{EmbedLinePaginator, PaginatorOptions};
use crate::{BotError, Context};
use diesel::{dsl, ExpressionMethods, QueryDsl};
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
    SnipeVictimRatio,
}

/// Show leaderboards by various sniping statistics
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Leaderboard type; default is \"Total snipes\'"] by: Option<LeaderboardBy>,
) -> Result<(), BotError> {
    let base_statement = message::table.inner_join(snipe::table);
    let mut conn = ctx.data().db_pool.get().await?;

    let by = by.unwrap_or_default();

    let lines = match by {
        LeaderboardBy::SnipeCount => {
            let count_expr = dsl::count_star();
            base_statement
                .group_by(message::author_id)
                .select((message::author_id, count_expr))
                .order_by(count_expr.desc())
                .load::<(i64, i64)>(&mut conn)
                .await?
                .into_iter()
                .enumerate()
                .map(|(i, (u_id, n))| {
                    format!("{}. {}: {}", i + 1, UserId::from(u_id as u64).mention(), n)
                        .into_boxed_str()
                })
                .collect_vec()
        }
        LeaderboardBy::VictimCount => {
            let count_expr = dsl::count_star();
            base_statement
                .group_by(snipe::victim_id)
                .select((snipe::victim_id, count_expr))
                .order_by(count_expr.desc())
                .load::<(i64, i64)>(&mut conn)
                .await?
                .into_iter()
                .enumerate()
                .map(|(i, (u_id, n))| {
                    format!("{}. {}: {}", i + 1, UserId::from(u_id as u64).mention(), n)
                        .into_boxed_str()
                })
                .collect_vec()
        }
        // TODO
        LeaderboardBy::SnipeVictimRatio => vec![Box::from("todo lol")],
    };

    let paginator = EmbedLinePaginator::new(
        lines,
        PaginatorOptions::default()
            .sep("\n")
            .max_lines(NonZeroUsize::new(10).unwrap())
            .reply(true)
            .ephemeral(true),
    );

    paginator.run(ctx).await?;
    Ok(())
}
