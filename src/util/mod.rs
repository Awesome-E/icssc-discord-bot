pub(crate) mod paginate;
pub(crate) mod text;

use crate::Context;
use serenity::all::{CreateEmbed, CreateEmbedAuthor, User};

pub(crate) fn base_embed(ctx: Context<'_>) -> CreateEmbed {
    CreateEmbed::default()
        .color(0xff87a6)
        .author(CreateEmbedAuthor::from(User::from(
            ctx.serenity_context().cache.current_user().clone(),
        )))
}
