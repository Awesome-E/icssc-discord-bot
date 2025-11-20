use crate::util::ContextExtras;
use crate::{AppError, AppVars, Context};
use anyhow::Context as _;
use entity::opt_out;
use poise::ChoiceParameter;
use sea_orm::{ActiveValue, DbErr};
use sea_orm::EntityTrait;
use serenity::all::{CacheHttp, ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, UserId};


pub(crate) struct SnipesOptOut<'a> {
    ctx: &'a serenity::client::Context,
    data: &'a AppVars,
}

impl<'a> SnipesOptOut<'a> {
    pub(crate) fn new(ctx: &'a serenity::client::Context, data: &'a AppVars) -> Self {
        Self { ctx, data }
    }

    async fn exists(&self, user_id: UserId) -> Result<bool, DbErr> {
        Ok(
            opt_out::Entity::find_by_id(u64::from(user_id) as i64)
                .one(&self.data.db)
                .await?
                .is_some(),
        )
    }

    pub(crate) async fn opt_out(&self, interaction: &ComponentInteraction) -> () {
        let opt_out_user = opt_out::ActiveModel {
            id: ActiveValue::Set(interaction.user.id.into())
        };

        let response = match self.exists(interaction.user.id).await {
            Err(_) => return,
            Ok(true) => "You are already opted out.",
            Ok(false) => match opt_out::Entity::insert(opt_out_user)
                .on_conflict_do_nothing()
                .exec(&self.data.db)
                .await
            {
                Ok(_) => "Successfully opted out of snipes!",
                Err(_) => "Unable to opt out. Please contact ICSSC IVP :(",
            },
        };

        let _ = interaction
            .create_response(
                self.ctx.http(),
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response)
                        .ephemeral(true),
                ),
            )
            .await;
    }

    pub(crate) async fn opt_in(&self, interaction: &ComponentInteraction) -> () {
        let response = match self.exists(interaction.user.id).await {
            Err(_) => return,
            Ok(false) => "You are already opted in to snipes!",
            Ok(true) => match opt_out::Entity::delete_by_id(u64::from(
                interaction.user.id,
            ) as i64)
            .exec(&self.data.db)
            .await
            {
                Ok(_) => "Successfully opted in to snipes!",
                Err(_) => "Unable to opt in. Please contact ICSSC IVP :(",
            },
        };

        let _ = interaction
            .create_response(
                self.ctx.http(),
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response)
                        .ephemeral(true),
                ),
            )
            .await;
    }

    pub(crate) async fn check(&self, interaction: &ComponentInteraction) -> () {
        let response = match self.exists(interaction.user.id).await {
            Ok(true) => "You are currently opted out of snipes!",
            Ok(false) => "You are currently opted into snipes!",
            Err(_) => "I couldn't check that for you :(",
        };

        let _ = interaction
            .create_response(
                self.ctx.http(),
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response)
                        .ephemeral(true),
                ),
            )
            .await;
    }
}

/// See whether you're opted out of being sniped
#[poise::command(prefix_command, slash_command)]
pub(crate) async fn check_snipes_participation(ctx: Context<'_>) -> Result<(), AppError> {
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
pub(crate) async fn set_snipes_participation(
    ctx: Context<'_>,
    #[description = "New value you want to set"] target: OptInStatus,
) -> Result<(), AppError> {
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
