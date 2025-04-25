-- Check not null status on arrays as per https://stackoverflow.com/a/59421233
--
-- Diesel assumes all arrays are allowed to contain null elements and throws a
-- temper tantrum if you don't account for it, see
-- https://github.com/diesel-rs/diesel/discussions/3310
--
-- however, having to make my own helper type or deal with more fucking options
-- is more work. editing schema.rs is easier
CREATE TABLE bulk_imports
(
    id       uuid PRIMARY KEY,
    name     text      NOT NULL,
    creator  uuid      NOT NULL,
    started  timestamp NOT NULL,
    finished timestamp,
    pending  text[]    NOT NULL check (array_position(pending, null) is null),
    accepted uuid[]    NOT NULL check (array_position(accepted, null) is null),
    rejected text[]    NOT NULL check (array_position(rejected, null) is null)
);
