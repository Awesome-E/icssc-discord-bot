use crate::util::ContextExtras;
use crate::{BotError, Context};
use anyhow::Context as _;
use entity::opt_out;
use poise::ChoiceParameter;
use sea_orm::ActiveValue;
use sea_orm::EntityTrait;

#[poise::command(prefix_command, slash_command, subcommands("status", "set"))]
pub(crate) async fn opt_out(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}

/// See whether you're opted out of being sniped
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn status(ctx: Context<'_>) -> Result<(), BotError> {
    let got = opt_out::Entity::find_by_id(ctx.author().id.get() as i64)
        .one(&ctx.data().db)
        .await
        .context("get opt out user id")?;

    ctx.reply_ephemeral(format!(
        "you are opted **{}** snipes",
        match got {
            None => "in to",
            Some(..) => "out of",
        }
    ))
    .await?;
    Ok(())
}

#[derive(ChoiceParameter, PartialEq, Eq, Copy, Clone, Debug, Hash)]
enum OptInStatus {
    #[name = "Opt in"]
    OptIn,
    #[name = "Opt out"]
    OptOut,
}

/// Opt in or out of being sniped
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn set(
    ctx: Context<'_>,
    #[description = "New value you want to set"] target: OptInStatus,
) -> Result<(), BotError> {
    let conn = &ctx.data().db;

    match target {
        OptInStatus::OptIn => {
            opt_out::Entity::delete_by_id(ctx.author().id.get() as i64)
                .exec(conn)
                .await
                .context("Opt out delete user id")?;

            ctx.reply_ephemeral("ok, you are now opted in; snipes including you can be logged!")
                .await?;
        }
        OptInStatus::OptOut => {
            let mdl = opt_out::ActiveModel {
                id: ActiveValue::Set(ctx.author().id.get() as i64),
            };

            opt_out::Entity::insert(mdl)
                .on_conflict_do_nothing()
                .exec(conn)
                .await?;
            ctx.reply_ephemeral("ok, you are now opted out; nobody can log a snipe including you")
                .await
                .context("opt out insert user id")?;
        }
    }

    Ok(())
}
