use crate::AppVars;
use crate::matchy::opt_in::MatchyMeetupOptIn;
use crate::spottings::snipe::confirm_message_snipe_modal;
use crate::util::text::bot_invite_url;
use rand::seq::IndexedRandom;
use serenity::all::{
    ActivityData, ActivityType, Context, EventHandler, Interaction, OnlineStatus, Permissions,
    Ready,
};
use serenity::async_trait;
use std::time::Duration;
use tokio::time;

pub(crate) struct LaikaEventHandler {
    pub(crate) data: AppVars,
}

#[async_trait]
impl EventHandler for LaikaEventHandler {
    async fn ready(&self, ctx: Context, ready_info: Ready) {
        println!(
            "ok, connected as {} (UID {})",
            ready_info.user.tag(),
            ready_info.user.id
        );
        println!("using discord API version {}", ready_info.version);
        println!(
            "invite link: {}",
            bot_invite_url(ready_info.user.id, Permissions::empty(), true)
        );

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(120));

            let status = [
                "spotting ICSSC members",
                "getting countersniped",
                "sneaking around",
                "taking out my phone",
                "sign up for matchy meetups!",
                "setting up matchy meetups",
                "visit icssc.club!",
                "come to ICSSC events!",
                "you can just build things",
                "you can just do things",
                "you can just spot people",
            ];

            loop {
                ctx.shard.set_presence(
                    Some(ActivityData {
                        name: String::from("bazinga"),
                        kind: ActivityType::Custom,
                        state: Some(String::from(*status.choose(&mut rand::rng()).unwrap())),
                        url: None,
                    }),
                    OnlineStatus::Idle,
                );
                interval.tick().await;
            }
        });
        println!("status cycling active");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Component(interaction) => match interaction.data.custom_id.as_str() {
                // TODO consider creating enums for custom IDs to avoid magic strings
                "matchy_opt_in" => {
                    MatchyMeetupOptIn::new(&ctx, &self.data)
                        .join(&interaction)
                        .await
                }
                "matchy_opt_out" => {
                    MatchyMeetupOptIn::new(&ctx, &self.data)
                        .leave(&interaction)
                        .await
                }
                "matchy_check_participation" => {
                    MatchyMeetupOptIn::new(&ctx, &self.data)
                        .check(&interaction)
                        .await
                }
                _ => (),
            },
            Interaction::Modal(interaction) => match interaction.data.custom_id.as_str() {
                "spotting_modal_confirm" => {
                    let _ = confirm_message_snipe_modal(ctx, &self.data, interaction).await;
                }
                _ => (),
            },
            _ => (),
        }
    }
}
