use crate::util::base_embed;
use crate::{BotError, Context};
use itertools::Itertools;
use poise::CreateReply;
use serenity::all::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponseMessage, ReactionType,
};
use serenity::builder::CreateInteractionResponse;
use std::borrow::Borrow;
use std::num::NonZeroUsize;
use std::time::Duration;

pub(crate) struct EmbedFieldPaginator {
    pages: Vec<Vec<(Box<str>, Box<str>)>>,
    footer: Box<str>,
    current_page: u8,
}

pub(crate) struct PaginatorOptions {
    // cap at 25
    max_fields: Option<usize>,
    footer: Box<str>,
}

impl Default for PaginatorOptions {
    fn default() -> Self {
        Self {
            max_fields: Some(25),
            footer: Box::from(""),
        }
    }
}

impl PaginatorOptions {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn max_fields(mut self, max_fields: impl Into<NonZeroUsize>) -> Self {
        self.max_fields = Some(max_fields.into().get());
        self
    }

    pub(crate) fn footer(mut self, footer: impl Borrow<str>) -> Self {
        self.footer = Box::from(footer.borrow());
        self
    }
}

impl EmbedFieldPaginator {
    pub(crate) fn new(
        fields: impl Iterator<Item = (impl Borrow<str>, impl Borrow<str>)>,
        options: PaginatorOptions,
    ) -> EmbedFieldPaginator {
        let mut chunks: Vec<Vec<(Box<str>, Box<str>)>> = Vec::new();
        chunks.push(vec![]);
        let mut working_chunk = &mut chunks[0];
        let mut num_in_working_chunk = 0usize;
        let mut num_chunks = 1usize;

        for field in fields {
            if num_in_working_chunk >= options.max_fields.unwrap_or(25) {
                chunks.push(vec![]);
                num_chunks += 1;
                working_chunk = &mut chunks[num_chunks - 1];
                num_in_working_chunk = 0;
            }

            working_chunk.push((Box::from(field.0.borrow()), Box::from(field.1.borrow())));
            num_in_working_chunk += 1;
        }

        Self {
            pages: chunks,
            footer: options.footer,
            current_page: 1,
        }
    }

    fn embed_for(&self, ctx: Context<'_>, page: u8) -> CreateEmbed {
        let pages = self.pages[(page - 1) as usize]
            .iter()
            .map(|(n, v)| (n.clone(), v.clone(), false))
            .collect_vec();
        base_embed(ctx)
            .fields(pages)
            .footer(CreateEmbedFooter::new(format!(
                "{page}/{} {}",
                self.pages.len(),
                self.footer,
            )))
    }

    pub(crate) async fn run(mut self, ctx: Context<'_>) -> Result<(), BotError> {
        let components = if self.pages.len() > 1 {
            vec![CreateActionRow::Buttons(vec![
                CreateButton::new("embedinator_start")
                    .emoji(ReactionType::Unicode(String::from("⏮️"))),
                CreateButton::new("embedinator_previous")
                    .emoji(ReactionType::Unicode(String::from("◀️"))),
                CreateButton::new("embedinator_next")
                    .emoji(ReactionType::Unicode(String::from("▶️"))),
                CreateButton::new("embedinator_end")
                    .emoji(ReactionType::Unicode(String::from("⏭️"))),
                CreateButton::new("embedinator_stop")
                    .emoji(ReactionType::Unicode(String::from("⏹️"))),
            ])]
        } else {
            vec![]
        };

        let sent_handle = ctx
            .send(
                CreateReply::default()
                    .embed(self.embed_for(ctx, self.current_page))
                    .reply(true)
                    .components(components),
            )
            .await?;

        if self.pages.len() <= 1 {
            return Ok(());
        }

        let sent_message = sent_handle.message().await?;
        while let Some(ixn) = sent_message
            .await_component_interaction(&ctx.serenity_context().shard)
            .author_id(ctx.author().id)
            .timeout(Duration::from_secs(120))
            .await
        {
            match ixn.data.custom_id.as_str() {
                "embedinator_start" => {
                    self.current_page = 1;
                    ixn.create_response(
                        ctx.http(),
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .embed(self.embed_for(ctx, self.current_page)),
                        ),
                    )
                    .await?;
                }
                "embedinator_previous" => {
                    self.current_page -= 1;
                    if self.current_page == 0 {
                        self.current_page = self.pages.len() as u8
                    }
                    ixn.create_response(
                        ctx.http(),
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .embed(self.embed_for(ctx, self.current_page)),
                        ),
                    )
                    .await?;
                }
                "embedinator_next" => {
                    self.current_page += 1;
                    if self.current_page > self.pages.len() as u8 {
                        self.current_page = 1
                    }
                    ixn.create_response(
                        ctx.http(),
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .embed(self.embed_for(ctx, self.current_page)),
                        ),
                    )
                    .await?;
                }
                "embedinator_end" => {
                    self.current_page = self.pages.len() as u8;
                    ixn.create_response(
                        ctx.http(),
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .embed(self.embed_for(ctx, self.current_page)),
                        ),
                    )
                    .await?;
                }
                "embedinator_stop" => break,
                _ => {}
            }
        }

        sent_handle
            .edit(
                ctx,
                CreateReply::default()
                    .embed(self.embed_for(ctx, self.current_page))
                    .components(vec![]),
            )
            .await?;

        Ok(())
    }
}
