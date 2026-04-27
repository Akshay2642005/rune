-- 001_init.sql
-- Rune v0.2.0 schema

CREATE TABLE IF NOT EXISTS functions (
    id          TEXT    PRIMARY KEY,
    subdomain   TEXT    UNIQUE,
    route       TEXT    UNIQUE NOT NULL,
    wasm_path   TEXT    NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_functions_route     ON functions(route);
CREATE INDEX IF NOT EXISTS idx_functions_subdomain ON functions(subdomain);

-- api_keys stores the SHA-256 hex hash of the raw key, never the key itself.
-- The raw key is printed once on creation and never stored.
CREATE TABLE IF NOT EXISTS api_keys (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    key_hash    TEXT    NOT NULL UNIQUE,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
