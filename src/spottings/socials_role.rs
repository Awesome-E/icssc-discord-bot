use anyhow::bail;
use serenity::all::{
    CacheHttp as _, ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
    Member, RoleId,
};

use crate::{AppError, AppVars};

pub(crate) struct SocialsParticipation<'a> {
    ctx: &'a serenity::client::Context,
    role_id: RoleId,
}

impl<'a> SocialsParticipation<'a> {
    pub(crate) fn new(ctx: &'a serenity::client::Context, data: &'a AppVars) -> Self {
        let role_id = RoleId::from(data.roles.socials_role_id);
        Self { ctx, role_id }
    }

    async fn has_role(&self, member: &Member) -> Result<bool, AppError> {
        Ok(member.roles.contains(&self.role_id))
    }

    pub(crate) async fn opt_out(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let Some(member) = &interaction.member else {
            bail!("unexpected non-guild interaction");
        };

        let response = match self.has_role(member).await {
            Err(_) => bail!("Unable to find you. Please contact ICSSC IVP :("),
            Ok(false) => bail!("You are already opted out."),
            Ok(true) => match member.remove_role(self.ctx.http(), self.role_id).await {
                Ok(_) => "Successfully opted out of socials :(",
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

    pub(crate) async fn opt_in(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let Some(member) = &interaction.member else {
            bail!("unexpected non-guild interaction");
        };

        let response = match self.has_role(member).await {
            Err(_) => bail!("Unable to find you. Please contact ICSSC IVP :("),
            Ok(true) => bail!("You are already opted in to socials!"),
            Ok(false) => match member.add_role(self.ctx.http(), self.role_id).await {
                Ok(_) => "Successfully opted in to socials!",
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

    pub(crate) async fn check(&self, interaction: &ComponentInteraction) -> anyhow::Result<()> {
        let Some(member) = &interaction.member else {
            bail!("unexpected non-guild interaction");
        };

        let response = match self.has_role(member).await {
            Ok(true) => "You are currently opted in to socials!",
            Ok(false) => "You are currently opted out of socials!",
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
