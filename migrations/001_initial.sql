CREATE TABLE IF NOT EXISTS sessions (
    id          TEXT NOT NULL PRIMARY KEY,
    claim       TEXT NOT NULL,
    verdict     TEXT,
    reasoning   TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS turns (
    id          TEXT NOT NULL PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions(id),
    role        TEXT NOT NULL,
    content     TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
