use sea_orm::{ActiveValue, DbErr, EntityTrait};
use serenity::all::{CacheHttp, ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, UserId};
use serenity::client::Context;
use entity::{matchy_meetup_opt_in};
use crate::{BotVars};

pub(crate) struct MatchyMeetupOptIn<'a> {
    ctx: &'a Context,
    data: &'a BotVars,
}

impl<'a> MatchyMeetupOptIn<'a> {
    pub(crate) fn new(ctx: &'a Context, data: &'a BotVars) -> Self {
        Self {
            ctx,
            data,
        }
    }

    async fn exists(&self, user_id: UserId) -> Result<bool, DbErr> {
        Ok(matchy_meetup_opt_in::Entity::find_by_id(u64::from(user_id) as i64).one(&self.data.db).await?.is_some())
    }

    pub(crate) async fn join(&self, interaction: &ComponentInteraction) -> () {
        let participant = matchy_meetup_opt_in::ActiveModel {
            user_id: ActiveValue::Set(interaction.user.id.into()),
            created_at: Default::default(),
        };

        let response = match self.exists(interaction.user.id).await {
            Err(_) => return,
            Ok(true) => "You are already opted in.",
            Ok(false) => match matchy_meetup_opt_in::Entity::insert(participant).on_conflict_do_nothing().exec(&self.data.db).await {
                Ok(_) => "Successfully opted in to Matchy Meetups!",
                Err(_) => "Unable to opt in. Please contact ICSSC IVP :("
            }
        };

        let _ = interaction.create_response(
            self.ctx.http(),
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
                response).ephemeral(true))).await;
    }

    pub(crate) async fn leave(&self, interaction: &ComponentInteraction) -> () {
        let response = match self.exists(interaction.user.id).await {
            Err(_) => return,
            Ok(false) => "You are not in Matchy Meetups!",
            Ok(true) => match matchy_meetup_opt_in::Entity::delete_by_id(u64::from(interaction.user.id) as i64).exec(&self.data.db).await {
                Ok(_) => "Successfully opted out of Matchy Meetups!",
                Err(_) => "Unable to opt out. Please contact ICSSC IVP :("
            }
        };

        let _ = interaction.create_response(
            self.ctx.http(),
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
                response).ephemeral(true))).await;
    }

    pub(crate) async fn check(&self, interaction: &ComponentInteraction) -> () {
        println!("User wants to check opt in status");

        let response = match self.exists(interaction.user.id).await {
            Ok(true) => "You are currently opted in to Matchy Meetups!",
            Ok(false) => "You are currently opted out of Matchy Meetups!",
            Err(_) => "I couldn't check that for you :(",
        };

        let _ = interaction.create_response(
            self.ctx.http(),
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
                response).ephemeral(true))).await;
    }
}

