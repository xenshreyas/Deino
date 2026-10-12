-- DEV-01: outbound webhooks (ADR 2026-10-12-webhook-deliveries).
--
-- `webhooks` holds the endpoints the user added in Settings → Developer →
-- Webhooks. Each endpoint's signing secret lives in the Keychain
-- (`integration.webhook.<id>`), never here. `webhook_deliveries` holds one
-- row per message to one endpoint across all its retries: the dispatcher
-- picks up `pending` rows whose `next_attempt_at` has come, also after a
-- restart, and the page's delivery log reads the newest rows.
--
-- JSON columns (§3A): `events_json` (`Vec<WebhookEvent>`) and `fields_json`
-- (`PayloadFields`) are parts of one endpoint, read and written whole and
-- never queried by field.
--
-- Conventions as in FND-07: STRICT, NOT NULL by default, timestamps RFC 3339
-- UTC, an index for every foreign key and list order.

-- An outbound webhook endpoint: where to POST, for which events, with which
-- payload fields, and its failure streak (auto-disable).
CREATE TABLE webhooks (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- https, or http on a loopback host (testing).
    url TEXT NOT NULL CHECK (length(url) BETWEEN 8 AND 2048),
    -- What the user calls it; may be empty.
    label TEXT NOT NULL CHECK (length(label) <= 80),
    events_json TEXT NOT NULL
        CHECK (json_valid(events_json) AND json_type(events_json) = 'array'),
    fields_json TEXT NOT NULL
        CHECK (json_valid(fields_json) AND json_type(fields_json) = 'object'),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    -- Why Deino turned it off: `failing` (failed for a day) or `gone`
    -- (answered 410); NULL while on, or when the user turned it off.
    disabled_reason TEXT CHECK (disabled_reason IN ('failing', 'gone')),
    -- Attempts failed in a row since the last success, and when the streak
    -- began; reset by a success or by turning it on again.
    failure_streak INTEGER NOT NULL CHECK (failure_streak >= 0),
    failing_since TEXT,
    last_success_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- The Webhooks page lists them oldest first.
CREATE INDEX idx_webhooks_created ON webhooks (created_at, id);

-- One message to one endpoint, kept across its retries; the delivery log.
CREATE TABLE webhook_deliveries (
    -- Also the `webhook-id` header (`msg_` and the UUID's hex), the same on
    -- every retry so receivers can de-duplicate.
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    webhook_id TEXT NOT NULL REFERENCES webhooks (id) ON DELETE CASCADE,
    event TEXT NOT NULL CHECK (event IN (
        'meeting.ready', 'notes.generated', 'notes.edited',
        'action_item.created', 'action_item.updated', 'insight.generated',
        'automation', 'test'
    )),
    meeting_id TEXT REFERENCES meetings (id) ON DELETE SET NULL,
    -- The action item or insight the event is about.
    subject_id TEXT,
    -- An automation step's `run:<run>:<position>`: one delivery per key and
    -- endpoint, so a retried step never posts twice.
    idempotency_key TEXT CHECK (idempotency_key IS NULL OR length(idempotency_key) <= 200),
    -- `pending`: waiting for `next_attempt_at`; `sending`: an attempt is in
    -- flight (reset to `pending` at launch); `succeeded`; `failed`: gave up.
    status TEXT NOT NULL CHECK (status IN ('pending', 'sending', 'succeeded', 'failed')),
    attempts INTEGER NOT NULL CHECK (attempts >= 0),
    -- The JSON body, fixed at the first attempt so every retry sends the
    -- same bytes; cleared once the delivery ends.
    body TEXT,
    next_attempt_at TEXT CHECK ((status = 'pending') = (next_attempt_at IS NOT NULL)),
    -- The last attempt's HTTP status, and its user-safe error.
    last_status INTEGER,
    last_error TEXT,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    updated_at TEXT NOT NULL
) STRICT;

-- An automation step's delivery is found by its key.
CREATE UNIQUE INDEX idx_webhook_deliveries_key ON webhook_deliveries (webhook_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
-- The delivery log, newest first, and the cascade.
CREATE INDEX idx_webhook_deliveries_webhook ON webhook_deliveries (webhook_id, created_at);
-- The dispatcher's due queue.
CREATE INDEX idx_webhook_deliveries_due ON webhook_deliveries (next_attempt_at)
    WHERE status = 'pending';
-- The meeting foreign key's SET NULL.
CREATE INDEX idx_webhook_deliveries_meeting ON webhook_deliveries (meeting_id)
    WHERE meeting_id IS NOT NULL;
