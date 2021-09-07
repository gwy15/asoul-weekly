-- Add up migration script here
ALTER TABLE `item`
ADD COLUMN `marker` TEXT DEFAULT NULL;
