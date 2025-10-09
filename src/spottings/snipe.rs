use crate::util::paginate::{EmbedLinePaginator, PaginatorOptions};
use crate::util::text::comma_join;
use crate::util::{spottings_embed, ContextExtras};
use crate::{BotError, Context};
use anyhow::Context as _;
use entity::{message, opt_out, snipe};
use itertools::Itertools;
use poise::{ChoiceParameter, CreateReply};
use sea_orm::QueryFilter;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryOrder, TransactionTrait,
};
use serenity::all::{
    CreateActionRow, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage,
    Mentionable, ReactionType, User, UserId,
};
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::time::Duration;

#[derive(ChoiceParameter)]
enum SpottingType {
    Social,
    Snipe,
}

#[poise::command(prefix_command, slash_command, subcommands("post", "log"))]
pub(crate) async fn spotting(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.reply("base command is a noop").await?;
    Ok(())
}

/// Log a social or snipe
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn post(
    ctx: Context<'_>,
    #[description = "Link to message with proof"] message: serenity::all::Message,
    #[description = "Was this a social or a snipe?"] r#type: SpottingType,
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
    .flatten()
    .collect::<HashSet<_>>();

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

    let conn = &ctx.data().db;

    let got = opt_out::Entity::find()
        .filter(opt_out::Column::Id.is_in(victims.iter().map(|v| v.id.get() as i64).collect_vec()))
        .all(conn)
        .await
        .context("log snipe get opt out user id")?;

    if !got.is_empty() && matches!(r#type, SpottingType::Snipe) {
        ctx.send(CreateReply::default().embed(spottings_embed().description(format!(
            "**the following people in that post are opted out of sniping!**\n{}\n\nthis means they do not consent to being photographed!",
            got.into_iter().map(|opted_out| UserId::new(opted_out.id as u64).mention()).join("\n"),
        ))).reply(true).ephemeral(true)).await?;
        return Ok(());
    }

    let emb = spottings_embed().description(format!(
        "**you are claiming that {} spotted**:\n{}\n\nclick to confirm! (times out in 15 seconds)",
        message.author.mention(),
        victims.iter().join("")
    ));

    let post_confirm_id = "spotting_post_confirm";

    let handle = ctx
        .send(
            CreateReply::default()
                .embed(emb.clone())
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(post_confirm_id)
                        .emoji(ReactionType::Unicode(String::from("😎"))),
                ])])
                .reply(true)
                .ephemeral(true),
        )
        .await?;

    let waited = match handle
        .message()
        .await?
        .await_component_interaction(&ctx.serenity_context().shard)
        .author_id(ctx.author().id)
        .custom_ids(vec![String::from(post_confirm_id)])
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

    let message_sql = message::ActiveModel {
        // command is guild_only
        guild_id: ActiveValue::Set(ctx.guild_id().unwrap().into()),
        channel_id: ActiveValue::Set(message.channel_id.into()),
        message_id: ActiveValue::Set(message.id.into()),
        author_id: ActiveValue::Set(message.author.id.into()),
        time_posted: ActiveValue::NotSet,
        is_social: ActiveValue::Set(match r#type {
            SpottingType::Social => true,
            SpottingType::Snipe => false,
        }),
    };

    let snipes_sql = victims
        .into_iter()
        .map(|victim| snipe::ActiveModel {
            message_id: ActiveValue::Set(message.id.into()),
            victim_id: ActiveValue::Set(victim.id.into()),
            latitude: ActiveValue::Set(None),
            longitude: ActiveValue::Set(None),
            notes: ActiveValue::Set(None),
        })
        .collect_vec();

    let Ok(_) = conn
        .transaction::<_, (), DbErr>(move |txn| {
            Box::pin(async move {
                message::Entity::insert(message_sql).exec(txn).await?;

                snipe::Entity::insert_many(snipes_sql).exec(txn).await?;

                txn.execute_unprepared("REFRESH MATERIALIZED VIEW user_stat")
                    .await?;

                Ok(())
            })
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

// #[derive(FromQueryResult)]
// struct ImplodedSnipes {
//     guild_id: i64,
//     channel_id: i64,
//     message_id: i64,
//     author_id: i64,
//     time_posted: DateTime,
//     first_name: String,
//     last_name: String,
//     victims: Vec<i64>,
// }

/// Log past snipes
#[poise::command(prefix_command, slash_command, guild_only)]
pub(crate) async fn log(ctx: Context<'_>) -> Result<(), BotError> {
    let conn = &ctx.data().db;

    let got = message::Entity::find()
        // .column_as(Expr::cust("array_agg(snipe.victim_id)"), "victims")
        .find_with_related(snipe::Entity)
        // .group_by(message::Column::MessageId)
        .order_by_desc(message::Column::MessageId)
        // .into_model::<ImplodedSnipes>()
        .all(conn)
        .await
        .context("log get recent snipes")?;

    let paginator = EmbedLinePaginator::new(
        got.iter()
            .map(|(msg, victims)| {
                format!(
                    "<t:{}:f>: **{}** sniped {} ([msg](https://discord.com/channels/{}/{}/{}))",
                    msg.time_posted.and_utc().timestamp(),
                    UserId::from(msg.author_id as u64).mention(),
                    comma_join(
                        victims
                            .iter()
                            .map(|victim| UserId::from(victim.victim_id as u64).mention())
                    ),
                    msg.guild_id,
                    msg.channel_id,
                    msg.message_id,
                )
                .into_boxed_str()
            })
            .collect_vec(),
        PaginatorOptions::default()
            .sep("\n\n")
            .max_lines(NonZeroUsize::new(10).unwrap())
            .ephemeral(true),
    );

    paginator.run(ctx).await.context("log paginate")?;

    Ok(())
}
