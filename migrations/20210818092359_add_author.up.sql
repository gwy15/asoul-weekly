-- Add up migration script here
ALTER TABLE `item`
ADD COLUMN `author` TEXT NOT NULL DEFAULT '<unknown>';
