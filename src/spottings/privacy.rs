use crate::util::ContextExtras as _;
use crate::{AppError, AppVars, AppContext};
use anyhow::{Context as _, bail, ensure};
use entity::snipe_opt_out;
use poise::ChoiceParameter;
use sea_orm::EntityTrait as _;
use sea_orm::{ActiveValue, DbErr};
use serenity::all::{
    CacheHttp as _, Channel, ChannelId, ChannelType, ComponentInteraction,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, GuildChannel,
    UserId,
};

pub(crate) struct SnipesOptOut<'a> {
    ctx: &'a serenity::all::Context,
    data: &'a AppVars,
}

impl<'a> SnipesOptOut<'a> {
    pub(crate) fn new(ctx: &'a serenity::all::Context, data: &'a AppVars) -> Self {
        Self { ctx, data }
    }

    async fn exists(&self, user_id: UserId) -> Result<bool, DbErr> {
        Ok(snipe_opt_out::Entity::find_by_id(u64::from(user_id) as i64)
            .one(&self.data.db)
            .await?
            .is_some())
    }

    async fn get_spottings_channel(&self) -> anyhow::Result<GuildChannel> {
        let ch_id = ChannelId::from(self.data.channels.spottings_channel_id);
        let channel = self.ctx.http().get_channel(ch_id).await?;
        let Channel::Guild(channel) = channel else {
            bail!("unexpected non-text channel");
        };
        ensure!(channel.kind == ChannelType::Text);

        Ok(channel)
    }

    pub(crate) async fn opt_out(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let opt_out_user = snipe_opt_out::ActiveModel {
            id: ActiveValue::Set(interaction.user.id.into()),
        };

        let response = match self.exists(interaction.user.id).await {
            Err(_) => bail!("Unable to find you. Please contact ICSSC IVP :("),
            Ok(true) => bail!("You are already opted out."),
            Ok(false) => match snipe_opt_out::Entity::insert(opt_out_user)
                .on_conflict_do_nothing()
                .exec(&self.data.db)
                .await
            {
                Ok(_) => "Successfully opted out of snipes!",
                Err(_) => bail!("Unable to opt out. Please contact ICSSC IVP :("),
            },
        };

        interaction
            .create_response(
                self.ctx.http(),
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response)
                        .ephemeral(true),
                ),
            )
            .await?;

        let channel = self.get_spottings_channel().await?;
        let content = format!("<@{}> has opted out of being sniped!", interaction.user.id);
        channel
            .send_message(self.ctx.http(), CreateMessage::new().content(content))
            .await?;

        Ok(())
    }

    pub(crate) async fn opt_in(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let response = match self.exists(interaction.user.id).await {
            Err(_) => bail!("Unable to find you. Please contact ICSSC IVP :("),
            Ok(false) => bail!("You are already opted in to snipes!"),
            Ok(true) => {
                match snipe_opt_out::Entity::delete_by_id(u64::from(interaction.user.id) as i64)
                    .exec(&self.data.db)
                    .await
                {
                    Ok(_) => "Successfully opted in to snipes!",
                    Err(_) => bail!("Unable to opt in. Please contact ICSSC IVP :("),
                }
            }
        };

        interaction
            .create_response(
                self.ctx.http(),
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response)
                        .ephemeral(true),
                ),
            )
            .await?;

        let channel = self.get_spottings_channel().await?;
        let content = format!("<@{}> has rejoined snipes!", interaction.user.id);
        channel
            .send_message(self.ctx.http(), CreateMessage::new().content(content))
            .await?;

        Ok(())
    }

    pub(crate) async fn check(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let response = match self.exists(interaction.user.id).await {
            Ok(true) => "You are currently opted out of snipes!",
            Ok(false) => "You are currently opted in to snipes!",
            Err(_) => bail!("I couldn't check that for you :("),
        };

        interaction
            .create_response(
                self.ctx.http(),
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response)
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }
}

/// See whether you're opted out of being sniped
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn check_snipes_participation(ctx: AppContext<'_>) -> Result<(), AppError> {
    let got = snipe_opt_out::Entity::find_by_id(ctx.author().id.get() as i64)
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
pub(crate) async fn set_snipes_participation(
    ctx: AppContext<'_>,
    #[description = "New value you want to set"] target: OptInStatus,
) -> Result<(), AppError> {
    let conn = &ctx.data().db;

    match target {
        OptInStatus::OptIn => {
            snipe_opt_out::Entity::delete_by_id(ctx.author().id.get() as i64)
                .exec(conn)
                .await
                .context("Opt out delete user id")?;

            ctx.reply_ephemeral("ok, you are now opted in; snipes including you can be logged!")
                .await?;
        }
        OptInStatus::OptOut => {
            let mdl = snipe_opt_out::ActiveModel {
                id: ActiveValue::Set(ctx.author().id.get() as i64),
            };

            snipe_opt_out::Entity::insert(mdl)
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
