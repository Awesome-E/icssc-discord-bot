use anyhow::Context as _;
use chrono::Utc;
use regex::Regex;
use reqwest::StatusCode;
use serde_json::json;
use crate::{AppError, Context, util::ContextExtras};

async fn reply_link_style_error(ctx: &Context<'_>, cause: &str) -> Result<(), AppError> {
    let guide_url = &ctx.data().env.shortlink.style_guide_url;
    ctx.reply_ephemeral(format!("Link shortener [style guide]({guide_url}) error: {cause}")).await?;
    Ok(())
}

async fn validate_identifier(ctx: &Context<'_>, identifier: &str) -> Result<bool, AppError> {
    let regex = Regex::new(&Utc::now().format(r"(\b|\D)(%Y|%y)").to_string())
        .context("invalid year regex")?;

    if regex.is_match(&identifier) {
        reply_link_style_error(ctx, "identifier should not contain current year").await?;
        return Ok(false);
    }

    if Regex::new(r"[a-z][A-Z]")?.is_match(&identifier) {
        reply_link_style_error(ctx, "identifier should use hyphens to distinguish words, not uppercase letters").await?;
        return Ok(false);
    }

    if Regex::new(r"[A-Z]")?.is_match(&identifier) {
        reply_link_style_error(ctx, "identifier should not contain uppercase letters").await?;
        return Ok(false);
    }

    if Regex::new(r"_")?.is_match(&identifier) {
        reply_link_style_error(ctx, "identifier should use dashes, not underscores").await?;
        return Ok(false);
    }

    if Regex::new(r"[^a-z0-9-]")?.is_match(&identifier) {
        reply_link_style_error(ctx, "identifier should not contain special characters").await?;
        return Ok(false);
    }

    Ok(true)
}

async fn reply_invalid_destination_error(ctx: &Context<'_>, cause: &str) -> Result<(), AppError> {
    ctx.reply_ephemeral(format!("Invalid destination url: {cause}")).await?;
    Ok(())
}

/// Create a new icssc.link short link
#[poise::command(slash_command, hide_in_help, ephemeral)]
pub(crate) async fn create(
    ctx: Context<'_>,
    #[description = "the identifier of the shortlink, e.g. committee-apps"]
    identifier: String,
    #[description = "The link to redirect to. Paste a full, unshortened link (NOT forms.gle, tinyurl, etc.)"]
    destination: String,
) -> Result<(), AppError> {
    ctx.defer_ephemeral().await?;

    if !validate_identifier(&ctx, &identifier).await? {
        return Ok(());
    };

    if reqwest::Url::parse(&destination).is_err() {
        return reply_invalid_destination_error(&ctx, "Destination must be a valid url").await;
    }
    
    // sometimes google form short links do not immediately redirect
    let is_short_gform_link = Regex::new(r"^https://forms.gle")?.is_match(&destination);
    if is_short_gform_link {
        return reply_invalid_destination_error(&ctx, "Destination should be a full url, not a short link").await;
    }

    let data = ctx.data();
    let client = &data.http.client;

    let attempted_destination_resp = client
        .get(&destination)
        .send()
        .await?;

    let is_google_link = Regex::new(r"https://([\w.]+)google.com/")?.is_match(&destination);
    let correct_url = match is_google_link {
        true => &destination,
        false => attempted_destination_resp.url().as_str()
    };

    // Would be nice to check the status to validate, but Google Forms for some reason
    // has a 401 status on the sign in redirect. I'd rather a wrong link get created (since it
    // can be overridden) than a correct link get blocked.

    // TODO url-encode the identifier
    let url = format!("https://icssc.link/");
    let response_status = client
        .post(url) // head requests are not supported by icssc.link
        .bearer_auth(&data.env.shortlink.secret)
        .json(&json!({
            "Identifier": identifier.as_str(),
            "Target": correct_url
        }))
        .send()
        .await?
        .status();

    let message = match response_status {
        StatusCode::OK => {
            format!("Successfully created redirect from https:\\//icssc.link/**{identifier}** to {correct_url}")
        },
        other_status => format!("Failed to create short link. Status: {other_status}")
    };

    ctx.reply_ephemeral(message).await?;

    Ok(())
}
