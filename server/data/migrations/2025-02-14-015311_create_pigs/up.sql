-- Enable pg_trgm extension
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Create the core pigs table
CREATE TABLE pigs
(
    id      uuid PRIMARY KEY,
    name    text UNIQUE NOT NULL,
    created timestamp   NOT NULL
);
