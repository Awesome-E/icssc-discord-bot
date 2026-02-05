use anyhow::{Context as _, bail};
use reqwest::StatusCode;
use urlencoding::encode;

use crate::{AppError, Context, util::ContextExtras as _};

/// Returns where the shortlink redirects to
#[poise::command(slash_command, hide_in_help, ephemeral)]
pub(crate) async fn check(
    ctx: Context<'_>,
    #[description = "the identifier of the shortlink, e.g. committee-apps"] identifier: String,
) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let url = format!("https://icssc.link/{}", encode(&identifier));
    let response = client
        .get(url) // head requests are not supported by icssc.link
        .send()
        .await?;

    let destination = match response.status() {
        StatusCode::MOVED_PERMANENTLY
        | StatusCode::FOUND
        | StatusCode::TEMPORARY_REDIRECT
        | StatusCode::PERMANENT_REDIRECT => response
            .headers()
            .get("location")
            .context("Cannot determine location")?
            .to_str()?,
        other_status => bail!("{other_status}"),
    };

    let response = format!("https:\\//icssc.link/**{identifier}** redirects to:\n{destination}");
    ctx.reply_ephemeral(response).await?;

    Ok(())
}
