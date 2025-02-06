use crate::model::{Message, Snipe};
use crate::schema::{message, snipe};
use crate::util::base_embed;
use crate::{BotError, Context};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use itertools::Itertools;
use poise::CreateReply;
use serenity::all::{CreateActionRow, CreateButton, CreateInteractionResponse, ReactionType, User};
use std::collections::{HashMap, HashSet};
use std::convert::identity;
use std::time::Duration;

#[poise::command(prefix_command, slash_command, subcommands("post", "log"))]
pub(crate) async fn snipe(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}

/// Log a snipe
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn post(
    ctx: Context<'_>,
    #[description = "Link to message with proof"] message: serenity::all::Message,
    #[description = "The first victim"] victim1: User,
    #[description = "Another victim, if applicable"] victim2: Option<User>,
    #[description = "Another victim, if applicable"] victim3: Option<User>,
    #[description = "Another victim, if applicable"] victim4: Option<User>,
    #[description = "Another victim, if applicable"] victim5: Option<User>,
    #[description = "Another victim, if applicable"] victim6: Option<User>,
    #[description = "Another victim, if applicable"] victim7: Option<User>,
    #[description = "Another victim, if applicable"] victim8: Option<User>,
    // #[description = "Another victim, if applicable"] victim9: Option<User>,
    // #[description = "Another victim, if applicable"] victim10: Option<User>,
) -> Result<(), BotError> {
    let victims = vec![
        Some(victim1),
        victim2,
        victim3,
        victim4,
        victim5,
        victim6,
        victim7,
        victim8,
        // victim9,
        // victim10,
    ]
        .into_iter()
        .filter_map(identity)
        .collect::<HashSet<_>>();

    if victims.iter().any(|v| v.id == ctx.author().id) {
        ctx.reply("sanity check: you can't snipe yourself!").await?;
        return Ok(());
    }

    if victims.iter().any(|v| v.bot) {
        ctx.reply("sanity check: bots don't have physical forms to snipe!")
            .await?;
        return Ok(());
    }

    if message.channel_id != ctx.channel_id() {
        ctx.reply("that message isn't in this channel...").await?;
        return Ok(());
    }

    if message
        .attachments
        .iter()
        .all(|attachment| attachment.height.is_none())
    {
        ctx.reply("no images in your linked message!").await?;
        return Ok(());
    }

    let handle = ctx
        .send(
            CreateReply::default()
                .embed(base_embed(ctx).description(format!(
                    "**you are claiming a snipe of**:\n{}\n\nclick to confirm! (times out in 15 seconds)",
                    victims.iter().join("")
                )))
                .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
                    "snipe_post_confirm",
                )
                    .emoji(ReactionType::Unicode(String::from("😎")))])])
                .reply(true)
                .ephemeral(true),
        )
        .await?;

    match handle
        .message()
        .await?
        .await_component_interaction(&ctx.serenity_context().shard)
        .author_id(ctx.author().id)
        .custom_ids(vec![String::from("snipe_post_confirm")])
        .timeout(Duration::from_secs(15))
        .await
    {
        None => {
            ctx.send(
                CreateReply::default()
                    .content("ok, nevermind then")
                    .reply(true)
                    .ephemeral(true),
            )
                .await?;
            return Ok(());
        }
        Some(ixn) => {
            ixn.create_response(ctx.http(), CreateInteractionResponse::Acknowledge)
                .await?
        }
    };

    let message_sql = Message {
        // command is guild_only
        guild_id: ctx.guild_id().unwrap().into(),
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
            diesel::insert_into(message::table)
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

/// Log past snipes
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn log(ctx: Context<'_>) -> Result<(), BotError> {
    let mut conn = ctx.data().db_pool.get().await?;

    let bins = message::table
        .inner_join(snipe::table)
        .select((Message::as_select(), Snipe::as_select()))
        .order(message::message_id.desc())
        .load::<(Message, Snipe)>(&mut conn)
        .await?
        .into_iter()
        .fold(HashMap::new(), |mut hm, (msg, snipe)| {
            hm.entry(msg).or_insert(Vec::with_capacity(1)).push(snipe);
            hm
        });

    dbg!(bins);

    Ok(())
}
