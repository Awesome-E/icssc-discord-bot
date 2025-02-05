use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::message)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
    guild_id: i64,
    channel_id: i64,
    message_id: i64,
    author_id: i64,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::snipe)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Snipe {
    pub message_id: i64,
    pub victim_id: i64,
    pub long_lat: (f64, f64),
    pub notes: String,
}

impl Snipe {
    #[inline]
    fn lat(&self) -> f64 {
        self.long_lat.1
    }

    #[inline]
    fn long(&self) -> f64 {
        self.long_lat.0
    }
}
