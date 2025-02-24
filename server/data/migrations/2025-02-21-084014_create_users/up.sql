-- Create the SQL table for storing user data
CREATE TABLE users
(
    id          uuid PRIMARY KEY,
    username    text      NOT NULL,
    groups      text[],
    created     timestamp NOT NULL,
    seen        timestamp NOT NULL,
    sso_subject text      NOT NULL,
    sso_issuer  text      NOT NULL,
    session_exp timestamp
);

-- Add a column to the pigs table to store the creator of each pig, using single
-- quotes allows us to specify a literal value. double quotes refers to a column
-- name, no quotes makes it think that's a number.
ALTER TABLE pigs
    ADD COLUMN creator uuid NOT NULL default '00000000000000000000000000000000';
