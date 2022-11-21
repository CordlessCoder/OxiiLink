CREATE TABLE IF NOT EXISTS pastes
(
    id                INTEGER PRIMARY KEY NOT NULL,
    views             INTEGER             NOT NULL DEFAULT 0,
    scrapes           INTEGER             NOT NULL DEFAULT 0,
    data              BYTEA               NOT NULL,
    allow_scraping    BOOLEAN             NOT NULL DEFAULT true,
    obfuscate         BOOLEAN             NOT NULL DEFAULT false,
    created_at timestamptz not null default now()
);
