use crate::model::{Message, Snipe};
use crate::schema::message::dsl::message as message_t;
use crate::schema::snipe;
use crate::{BotError, Context};
use diesel::associations::HasTable;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use itertools::Itertools;
use serenity::all::User;
use std::convert::identity;

#[poise::command(prefix_command, slash_command, subcommands("post"))]
pub(crate) async fn snipe(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}

/// Log a snipe
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn post(
    ctx: Context<'_>,
    #[description = "Link to message with proof"] message: serenity::all::Message,
    #[description = "The first victim"] victim1: User,
    #[description = "Another victim, if applicable"] victim2: Option<User>,
    #[description = "Another victim, if applicable"] victim3: Option<User>,
    #[description = "Another victim, if applicable"] victim4: Option<User>,
    #[description = "Another victim, if applicable"] victim5: Option<User>,
    // #[description = "Another victim, if applicable"] victim6: Option<User>,
    // #[description = "Another victim, if applicable"] victim7: Option<User>,
    // #[description = "Another victim, if applicable"] victim8: Option<User>,
    // #[description = "Another victim, if applicable"] victim9: Option<User>,
    // #[description = "Another victim, if applicable"] victim10: Option<User>,
) -> Result<(), BotError> {
    let victims = vec![
        Some(victim1),
        victim2,
        victim3,
        victim4,
        victim5,
        // victim6, victim7, victim8, victim9, victim10,
    ]
    .into_iter()
    .filter_map(identity)
    .collect_vec();

    let Some(guild_id) = message.guild_id.clone() else {
        ctx.reply("message must be in a guild; someone has to see it!")
            .await?;
        return Ok(());
    };

    if message
        .attachments
        .iter()
        .all(|attachment| attachment.height.is_none())
    {
        ctx.reply("no images in your linked message!").await?;
        return Ok(());
    }

    let message_sql = Message {
        guild_id: guild_id.into(),
        channel_id: message.channel_id.into(),
        message_id: message.id.into(),
        author_id: message.author.id.into(),
    };

    let snipes_sql = victims
        .into_iter()
        .map(|victim| Snipe {
            message_id: message.id.into(),
            victim_id: victim.id.into(),
            latitude: None,
            longitude: None,
            notes: None,
        })
        .collect_vec();

    let mut conn = ctx.data().db_pool.get().await?;

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        async move {
            diesel::insert_into(message_t::table())
                .values(message_sql)
                .execute(conn)
                .await?;

            for snipe in snipes_sql {
                diesel::insert_into(snipe::table)
                    .values(snipe)
                    .execute(conn)
                    .await?;
            }

            Ok(())
        }
        .scope_boxed()
    })
    .await?;

    ctx.reply("ok, logged").await?;
    Ok(())
}
