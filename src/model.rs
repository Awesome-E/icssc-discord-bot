use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::prelude::*;
use std::hash::{Hash, Hasher};

#[derive(Queryable, Selectable, Eq, PartialEq, Hash, Debug)]
#[diesel(table_name = crate::schema::message)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub author_id: i64,
    pub time_posted: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::message)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertMessage {
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub author_id: i64,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "https://discord.com/channels/{}/{}/{}",
            self.guild_id, self.channel_id, self.message_id
        )
    }
}

#[derive(Queryable, Selectable, Insertable, Debug)]
#[diesel(belongs_to(Message))]
#[diesel(table_name = crate::schema::snipe)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Snipe {
    pub message_id: i64,
    pub victim_id: i64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
}

impl PartialEq<Self> for Snipe {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id && self.victim_id == other.victim_id
    }
}

impl Eq for Snipe {}

impl Hash for Snipe {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message_id.hash(state);
        self.victim_id.hash(state);
    }
}
