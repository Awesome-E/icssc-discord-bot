use crate::util::text::bot_invite_url;
use rand::seq::IndexedRandom;
use serenity::all::{ActivityData, ActivityType, Context, EventHandler, OnlineStatus, Permissions, Ready};
use serenity::async_trait;
use std::time::Duration;
use tokio::time;

pub(crate) struct ICSSpottingsCouncilEventHandler;

#[async_trait]
impl EventHandler for ICSSpottingsCouncilEventHandler {
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
                "arguing over snipes",
                "taking out my phone",
                "sneaking around",
                "rainbolting spottings",
                "keeping score",
                "avenging my fallen comrades",
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
}
