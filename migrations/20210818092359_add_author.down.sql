-- Add down migration script here
ALTER TABLE `item`
DROP COLUMN `author`;
