pub(crate) mod add_event_test {
    use std::str::FromStr as _;

    use actix_web::{HttpResponse, Responder, get};
    use serenity::all::{CreateScheduledEvent, GuildId, ScheduledEventType, Timestamp};

    use crate::server::ExtractedAppData;

    #[get("/test-add-event")]
    async fn add_event_test(data: ExtractedAppData) -> crate::server::Result<impl Responder> {
        let event = CreateScheduledEvent::new(
            ScheduledEventType::External,
            "Event Name",
            Timestamp::from_str("2025-10-10T00:00:00Z").unwrap(),
        )
        .location("Bad")
        .end_time(Timestamp::from_str("2025-10-10T02:00:00Z").unwrap());
        data.discord_http
            .create_scheduled_event(GuildId::from(957408720088891473), &event, None)
            .await?;

        Ok(HttpResponse::Ok().body("ok"))
    }
}
