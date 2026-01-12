use crate::{AppError, Context, util::ContextExtras as _, util::calendar::generate_add_calendar_link};
use anyhow::{Context as _, anyhow};

/// Link Google Calendars to Discord!
#[poise::command(
    slash_command,
    rename = "calendar",
    subcommands("add_calendar", "list_calendars"),
    guild_only
)]
pub(crate) async fn calendar_command(_: Context<'_>) -> Result<(), AppError> {
    Ok(())
}

/// Add a calendar to the current server
#[poise::command(slash_command, rename = "add")]
pub(crate) async fn add_calendar(
    ctx: Context<'_>,
    #[description = "ID of the Google Calendar to add (usually in the form of an email address)"]
    calendar_id: String,
) -> Result<(), AppError> {
    let Context::Application(app_ctx) = ctx else {
        return Err(anyhow!("receive application command"));
    };

    let link = generate_add_calendar_link(ctx.data(), app_ctx.interaction, calendar_id)
        .context("generate add calendar link")?;

    let content =
        format!("To finish adding the calendar, please [authorize your Google account]({link}).");
    ctx.reply_ephemeral(content).await?;

    Ok(())
}

/// List calendars in the current server
#[poise::command(slash_command, rename = "list")]
pub(crate) async fn list_calendars(_ctx: Context<'_>) -> Result<(), AppError> {
    Ok(())
}
