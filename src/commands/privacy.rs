use crate::model::OptedOutUser;
use crate::schema::opt_out;
use crate::util::ContextExtras;
use crate::{BotError, Context};
use diesel::dsl::count_star;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

#[poise::command(prefix_command, slash_command, subcommands("status", "set"))]
pub(crate) async fn opt_in(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}

/// See whether you're opted out of being sniped
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn status(ctx: Context<'_>) -> Result<(), BotError> {
    let mut conn = ctx.data().db_pool.get().await?;

    let got = opt_out::table
        .select(count_star())
        .filter(opt_out::id.eq(ctx.author().id.get() as i64))
        .limit(1)
        .load::<i64>(&mut conn)
        .await?[0];

    ctx.reply_ephemeral(format!(
        "you are opted **{}** snipes",
        match got {
            0 => "in to",
            _ => "out of",
        }
    ))
    .await?;
    Ok(())
}

/// Opt in or out of being sniped
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn set(
    ctx: Context<'_>,
    #[description = "New value; true to be opted in"] target: bool,
) -> Result<(), BotError> {
    let mut conn = ctx.data().db_pool.get().await?;

    match target {
        true => {
            diesel::delete(opt_out::table::filter(
                opt_out::table,
                opt_out::id.eq(ctx.author().id.get() as i64),
            ))
            .execute(&mut conn)
            .await?;
            ctx.reply_ephemeral("ok, you are now opted in; snipes including you can be logged!")
                .await?;
        }
        false => {
            diesel::insert_into(opt_out::table)
                .values(OptedOutUser {
                    id: ctx.author().id.get() as i64,
                })
                .on_conflict_do_nothing()
                .execute(&mut conn)
                .await?;
            ctx.reply_ephemeral("ok, you are now opted out; nobody can log a snipe including you")
                .await?;
        }
    }

    Ok(())
}
