use serenity::all::Mentionable as _;

use crate::{
    AppError, AppContext,
    util::{
        ContextExtras as _,
        roster::{RosterSheetRow, get_user_from_discord},
    },
};

#[poise::command(context_menu_command = "Lookup Member", guild_only)]
pub(crate) async fn user_lookup(
    ctx: AppContext<'_>,
    user: serenity::all::User,
) -> Result<(), AppError> {
    let mention = user.mention();
    let row = get_user_from_discord(ctx.data(), None, user.name).await?;
    let Some(RosterSheetRow { name, email, .. }) = row else {
        ctx.reply_ephemeral("User is not an internal member")
            .await?;
        return Ok(());
    };

    ctx.reply_ephemeral(format!("{mention} is {name} ({email})"))
        .await?;

    Ok(())
}
