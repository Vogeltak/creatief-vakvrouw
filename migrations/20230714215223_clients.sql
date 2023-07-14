CREATE TABLE IF NOT EXISTS client
(
    id       INTEGER PRIMARY KEY NOT NULL,
    name     TEXT NOT NULL UNIQUE,
    address  TEXT NOT NULL,
    zip      TEXT NOT NULL
);

CREATE INDEX client_idx ON client(name);