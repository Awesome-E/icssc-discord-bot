use crate::AppVars;
use anyhow::bail;
use entity::matchy_meetup_opt_in;
use sea_orm::{ActiveValue, DbErr, EntityTrait as _};
use serenity::all::{
    CacheHttp as _, ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
    UserId,
};
use serenity::client::Context;

pub(crate) struct MatchyMeetupOptIn<'a> {
    ctx: &'a Context,
    data: &'a AppVars,
}

impl<'a> MatchyMeetupOptIn<'a> {
    pub(crate) fn new(ctx: &'a Context, data: &'a AppVars) -> Self {
        Self { ctx, data }
    }

    async fn exists(&self, user_id: UserId) -> Result<bool, DbErr> {
        Ok(
            matchy_meetup_opt_in::Entity::find_by_id(u64::from(user_id) as i64)
                .one(&self.data.db)
                .await?
                .is_some(),
        )
    }

    pub(crate) async fn join(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let participant = matchy_meetup_opt_in::ActiveModel {
            user_id: ActiveValue::Set(interaction.user.id.into()),
            created_at: Default::default(),
        };

        let response = match self.exists(interaction.user.id).await {
            Err(_) => bail!("Cannot read db"),
            Ok(true) => "You are already opted in.",
            Ok(false) => match matchy_meetup_opt_in::Entity::insert(participant)
                .on_conflict_do_nothing()
                .exec(&self.data.db)
                .await
            {
                Ok(_) => "Successfully opted in to Matchy Meetups!",
                Err(_) => bail!("Unable to opt in. Please contact ICSSC IVP :("),
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

        Ok(())
    }

    pub(crate) async fn leave(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let response = match self.exists(interaction.user.id).await {
            Err(_) => bail!("Unable to read db"),
            Ok(false) => "You are not in Matchy Meetups!",
            Ok(true) => match matchy_meetup_opt_in::Entity::delete_by_id(u64::from(
                interaction.user.id,
            ) as i64)
            .exec(&self.data.db)
            .await
            {
                Ok(_) => "Successfully opted out of Matchy Meetups!",
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

        Ok(())
    }

    pub(crate) async fn check(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let response = match self.exists(interaction.user.id).await {
            Ok(true) => "You are currently opted in to Matchy Meetups!",
            Ok(false) => "You are currently opted out of Matchy Meetups!",
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
