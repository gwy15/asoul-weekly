-- Add up migration script here
CREATE TABLE `group` (
    `name`      TEXT NOT NULL    PRIMARY KEY,
    `chat_id`   TEXT NOT NULL
);
