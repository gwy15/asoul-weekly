-- Add up migration script here

-- 动态或者视频
CREATE TABLE `item` (
    `id`            TEXT    NOT NULL    PRIMARY KEY,
    `json`          TEXT    NOT NULL,
    `message_id`    TEXT    NOT NULL,
    `create_time`   TEXT    NOT NULL, -- item 本身的创建时间
    -- 标记
    `category`      TEXT, -- NULLABLE
    `mark_time`     TEXT  -- NULLABLE
);
