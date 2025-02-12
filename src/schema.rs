// @generated automatically by Diesel CLI.

diesel::table! {
    message (message_id) {
        guild_id -> Int8,
        channel_id -> Int8,
        message_id -> Int8,
        author_id -> Int8,
        time_posted -> Timestamp,
    }
}

diesel::table! {
    opt_out (id) {
        id -> Int8,
    }
}

diesel::table! {
    snipe (message_id, victim_id) {
        message_id -> Int8,
        victim_id -> Int8,
        latitude -> Nullable<Float8>,
        longitude -> Nullable<Float8>,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    user_stat (id) {
        id -> Int8,
        snipe -> Int8,
        sniped -> Int8,
        snipe_rate -> Float8,
    }
}

diesel::joinable!(snipe -> message (message_id));

diesel::allow_tables_to_appear_in_same_query!(
    message,
    opt_out,
    snipe,
    user_stat,
);
