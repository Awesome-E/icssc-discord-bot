use serenity::all::Mentionable as _;

use crate::{
    AppError, Context,
    util::{
        ContextExtras as _,
        roster::{RosterSheetRow, get_user_from_discord},
    },
};

#[poise::command(context_menu_command = "Lookup Member", guild_only)]
pub(crate) async fn user_lookup(
    ctx: Context<'_>,
    user: serenity::all::User,
) -> Result<(), AppError> {
    let mention = user.mention();
    let row = get_user_from_discord(ctx.data(), None, user.name).await?;
    let RosterSheetRow { name, email, .. } = match row {
        Some(row) => row,
        None => {
            ctx.reply_ephemeral("User is not an internal member")
                .await?;
            return Ok(());
        }
    };
    ctx.reply_ephemeral(format!("{} is {} ({})", mention, name, email))
        .await?;

    Ok(())
}
