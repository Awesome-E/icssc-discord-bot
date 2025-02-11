use crate::model::{InsertMessage, Message, OptedOutUser, Snipe};
use crate::schema::{message, opt_out, snipe};
use crate::util::paginate::{EmbedLinePaginator, PaginatorOptions};
use crate::util::text::comma_join;
use crate::util::{base_embed, ContextExtras};
use crate::{BotError, Context};
use diesel::dsl::sql;
use diesel::pg::sql_types;
use diesel::sql_types::BigInt;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use itertools::Itertools;
use poise::CreateReply;
use serenity::all::{
    CreateActionRow, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage,
    Mentionable, ReactionType, User, UserId,
};
use std::collections::HashSet;
use std::convert::identity;
use std::num::NonZeroUsize;
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
        ctx.reply_ephemeral("sanity check: you can't snipe yourself!")
            .await?;
        return Ok(());
    }

    if victims.iter().any(|v| v.bot) {
        ctx.reply_ephemeral("sanity check: bots don't have physical forms to snipe!")
            .await?;
        return Ok(());
    }

    // if message.guild_id != ctx.guild_id() {
    //     ctx.reply("that message isn't in this guild...").await?;
    //     return Ok(());
    // }

    if message
        .attachments
        .iter()
        .all(|attachment| attachment.height.is_none())
    {
        ctx.reply_ephemeral("no images in your linked message!")
            .await?;
        return Ok(());
    }

    let mut conn = ctx.data().db_pool.get().await?;

    let got = opt_out::table
        .select(OptedOutUser::as_select())
        .filter(opt_out::id.eq_any(victims.iter().map(|v| v.id.get() as i64).collect_vec()))
        .load::<OptedOutUser>(&mut conn)
        .await?;

    if !got.is_empty() {
        ctx.send(CreateReply::default().embed(base_embed(ctx).description(format!(
            "**the following people in that post are opted out of sniping!**\n{}\n\nthis means they do not consent to being photographed!",
            got.into_iter().map(|opted_out| <OptedOutUser as Into<UserId>>::into(opted_out).mention()).join("\n"),
        ))).reply(true).ephemeral(true)).await?;
        return Ok(());
    }

    let emb = base_embed(ctx).description(format!(
        "**you are claiming that {} sniped**:\n{}\n\nclick to confirm! (times out in 15 seconds)",
        message.author.mention(),
        victims.iter().join("")
    ));
    let handle = ctx
        .send(
            CreateReply::default()
                .embed(emb.clone())
                .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
                    "snipe_post_confirm",
                )
                .emoji(ReactionType::Unicode(String::from("😎")))])])
                .reply(true)
                .ephemeral(true),
        )
        .await?;

    let waited = match handle
        .message()
        .await?
        .await_component_interaction(&ctx.serenity_context().shard)
        .author_id(ctx.author().id)
        .custom_ids(vec![String::from("snipe_post_confirm")])
        .timeout(Duration::from_secs(15))
        .await
    {
        None => {
            ctx.reply_ephemeral("ok, nevermind then").await?;
            return Ok(());
        }
        Some(ixn) => {
            // defer here?
            ixn
        }
    };

    let message_sql = InsertMessage {
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

    let Ok(_) = conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
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
        .await
    else {
        ctx.reply_ephemeral("couldn't insert; has this message been logged before?")
            .await?;
        return Ok(());
    };

    // remove "please react below..." and button
    waited
        .create_response(
            ctx.http(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(emb.clone())
                    .components(vec![]),
            ),
        )
        .await?;

    ctx.reply_ephemeral("ok, logged").await?;
    Ok(())
}

/// Log past snipes
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn log(ctx: Context<'_>) -> Result<(), BotError> {
    let mut conn = ctx.data().db_pool.get().await?;

    let got = message::table
        .inner_join(snipe::table)
        .select((
            Message::as_select(),
            sql::<sql_types::Array<BigInt>>("array_agg(snipe.victim_id)"),
        ))
        .group_by(message::message_id)
        .order(message::message_id.desc())
        .load::<(Message, Vec<i64>)>(&mut conn)
        .await?;

    let paginator = EmbedLinePaginator::new(
        got.iter()
            .map(|(msg, victim_ids)| {
                format!(
                    "<t:{}:f>: **{}** sniped {} ([msg]({}))",
                    msg.time_posted.and_utc().timestamp(),
                    UserId::from(msg.author_id as u64).mention(),
                    comma_join(
                        victim_ids
                            .iter()
                            .map(|id| UserId::from(*id as u64).mention())
                    ),
                    msg,
                )
                .into_boxed_str()
            })
            .collect_vec(),
        PaginatorOptions::default()
            .sep("\n\n")
            .max_lines(NonZeroUsize::new(10).unwrap())
            .ephemeral(true),
    );

    paginator.run(ctx).await?;

    Ok(())
}
