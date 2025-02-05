// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "point", schema = "pg_catalog"))]
    pub struct Point;
}

diesel::table! {
    message (message_id) {
        guild_id -> Int8,
        channel_id -> Int8,
        message_id -> Int8,
        author_id -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Point;

    snipe (message_id, victim_id) {
        message_id -> Int8,
        victim_id -> Int8,
        location -> Nullable<Point>,
        notes -> Nullable<Text>,
    }
}

diesel::joinable!(snipe -> message (message_id));

diesel::allow_tables_to_appear_in_same_query!(
    message,
    snipe,
);
