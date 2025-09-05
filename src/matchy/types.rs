use anyhow::Error;
pub type Data = ();
pub type Context<'a> = poise::Context<'a, Data, Error>;
