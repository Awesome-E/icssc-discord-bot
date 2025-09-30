use anyhow::Context;
use jsonwebtoken::Header;
use serde::{Deserialize, Serialize};
use serenity::all::CommandInteraction;

use crate::BotError;

#[derive(Serialize, Deserialize, Debug)]
struct AddCalendarInteractionTrigger {
    guild_id: u64,
    interaction_token: String,
    calendar_id: String,
}

pub(crate) fn generate_add_calendar_link(
    ixn: &CommandInteraction,
    calendar_id: String,
) -> Result<String, BotError> {
    let root_url = std::env::var("RAILWAY_PUBLIC_DOMAIN").context("Missing Domain")?;

    let jwt = AddCalendarInteractionTrigger {
        guild_id: ixn
            .guild_id
            .map(|v| v.get())
            .context("Generate oauth link => guild id")?,
        interaction_token: ixn.token.clone(),
        calendar_id,
    };

    let encoding_key = jsonwebtoken::EncodingKey::from_secret(
        std::env::var_os("JWT_SECRET")
            .context("Missing JWT_SECRET")?
            .as_encoded_bytes(),
    );

    let encoded = jsonwebtoken::encode(&Header::default(), &jwt, &encoding_key)
        .context("Generate oauth link => encode interaction")?;

    let url = reqwest::Client::new()
        .get(format!("{root_url}/oauth/start/google"))
        .query(&[
            ("interaction", encoded), // imo, not worth db logic to store tokens that are only valid for 15 mins
        ])
        .build()
        .context("Generate oauth link => build url")?
        .url()
        .to_string();

    Ok(url)
}
