-- Your SQL goes here
ALTER TABLE message ADD COLUMN time_posted TIMESTAMP NOT NULL GENERATED ALWAYS AS (to_timestamp((message_id / (2 ^ 22) + 1420070400000) / 1000)) STORED;