-- This file should undo anything in `up.sql`
DROP TABLE users;

ALTER TABLE pigs
    DROP COLUMN creator;
