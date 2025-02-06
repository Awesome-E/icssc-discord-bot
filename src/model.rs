use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::message)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub author_id: i64,
}

#[derive(Queryable, Selectable, Insertable)]
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
