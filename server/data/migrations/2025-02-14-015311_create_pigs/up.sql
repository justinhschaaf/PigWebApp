CREATE TABLE pigs
(
    id      uuid PRIMARY KEY,
    name    text UNIQUE NOT NULL,
    created timestamp   NOT NULL
)
