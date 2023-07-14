CREATE TABLE IF NOT EXISTS invoice
(
    id          INTEGER PRIMARY KEY NOT NULL,
    nummer      INTEGER NOT NULL UNIQUE,
    client      INTEGER NOT NULL,
    -- JSON serialized list of objects
    work_items  TEXT NOT NULL,
    subtotal    REAL NOT NULL,
    btw         REAL NOT NULL,
    total       REAL NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(client) REFERENCES client(id)
);

CREATE INDEX nummer_idx ON invoice(nummer);
CREATE INDEX date_idx ON invoice(created_at); 
