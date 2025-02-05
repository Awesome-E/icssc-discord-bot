// @generated automatically by Diesel CLI.

diesel::table! {
    messages (message_id) {
        guild_id -> Int8,
        channel_id -> Int8,
        message_id -> Int8,
        author -> Int8,
    }
}

diesel::table! {
    snipes (message_id, victim) {
        message_id -> Int8,
        victim -> Int8,
    }
}

diesel::joinable!(snipes -> messages (message_id));

diesel::allow_tables_to_appear_in_same_query!(
    messages,
    snipes,
);
