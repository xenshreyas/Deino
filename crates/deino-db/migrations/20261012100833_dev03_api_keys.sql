-- DEV-03: keys for the local REST API (ADR 2026-10-12-local-rest-api).
--
-- A key's token is `deino_<id without hyphens>_<secret>`; only an argon2id
-- hash of the secret is stored (`secret_hash`, a PHC string), so a lost
-- token cannot be shown again, only revoked and replaced. Revoked keys stay
-- listed (greyed) until the user deletes them from the list.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, an index for the list order.

-- Keys for the local REST API: a name, read or read-write, and only an argon2 hash of the secret.
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 60),
    -- `read`: GET only; `read_write`: every endpoint.
    scope TEXT NOT NULL CHECK (scope IN ('read', 'read_write')),
    secret_hash TEXT NOT NULL CHECK (secret_hash LIKE '$argon2id$%'),
    created_at TEXT NOT NULL,
    -- Updated at most once a minute, so a busy script does not write on
    -- every request.
    last_used_at TEXT,
    revoked_at TEXT
) STRICT;

-- Settings → Developer lists keys newest first.
CREATE INDEX idx_api_keys_created ON api_keys (created_at DESC, id DESC);
