use anyhow::Context as _;
use entity::snipe_opt_out;
use itertools::Itertools as _;
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
use serenity::all::{CacheHttp as _, CreateMessage, Mentionable as _, Message, UserId};

use crate::{AppError, AppVars};

pub(crate) async fn check_message_snipe_victim(
    ctx: &serenity::all::Context,
    data: &AppVars,
    msg: &Message,
) -> Result<(), AppError> {
    if msg.channel_id.get() != data.channels.spottings_channel_id {
        return Ok(());
    }

    let user_ids = msg.mentions.iter().map(|user| user.id.get());
    if user_ids.len() == 0 {
        return Ok(());
    }

    let mut opted_out_ids = snipe_opt_out::Entity::find()
        .filter(snipe_opt_out::Column::Id.is_in(user_ids))
        .all(&data.db)
        .await
        .context("failed to get opt outs")?
        .into_iter()
        .map(|model| UserId::from(model.id as u64).mention());

    if opted_out_ids.len() == 0 {
        return Ok(());
    }

    let warning_msg = CreateMessage::new().content(format!(
        "Heads up! In your [recent message]({}), you mentioned the following \
        users who are opted out of snipes: {}\n\n \
        If your message is a snipe, please remove the message. \
        Otherwise, feel free to disregard this notice.",
        msg.link(),
        opted_out_ids.join(", ")
    ));

    if let Err(why) = msg.author.direct_message(ctx.http(), warning_msg).await {
        dbg!(why);
        // TODO instead DM the owner of the bot
    }

    Ok(())
}
