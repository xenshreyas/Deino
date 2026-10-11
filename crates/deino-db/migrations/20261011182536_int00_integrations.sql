-- INT-00: the integrations framework (ADR 0029).
--
-- Tokens never live here: each connection's credential is one Keychain item
-- (`deino-secrets` `CredentialKey`, `integration.<provider>.<connection id>`).
-- These tables hold what the UI may show and what the delivery engine needs
-- to stay idempotent.

-- 1. New integration services. `egress_log.service` lists every
--    `deino_domain::ServiceId`, so new services rebuild the table (FND-12's
--    migration says so). Rows are copied unchanged, in rowid order.
CREATE TABLE egress_log_next (
    id INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    service TEXT NOT NULL CHECK (service IN (
        'llm_anthropic', 'llm_openai', 'llm_groq', 'llm_openrouter', 'llm_custom',
        'llm_ollama', 'hf_models', 'google_calendar', 'google_drive', 'gmail',
        'microsoft_graph', 'slack', 'notion', 'linear', 'todoist', 'asana',
        'hubspot', 'attio', 'salesforce', 'jira', 'monday', 'clickup', 'trello',
        'affinity', 'zoho_crm', 'web_search', 'webhooks'
    )),
    host TEXT NOT NULL CHECK (host <> '' AND instr(host, '/') = 0 AND instr(host, '?') = 0),
    method TEXT NOT NULL CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE')),
    status INTEGER CHECK (status BETWEEN 100 AND 599),
    outcome TEXT NOT NULL CHECK (outcome IN ('sent', 'failed', 'blocked')),
    bytes_up INTEGER NOT NULL CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL CHECK (bytes_down >= 0)
) STRICT;

INSERT INTO egress_log_next (id, at, service, host, method, status, outcome, bytes_up, bytes_down)
SELECT id, at, service, host, method, status, outcome, bytes_up, bytes_down
FROM egress_log
ORDER BY id;

DROP TABLE egress_log;
ALTER TABLE egress_log_next RENAME TO egress_log;

-- 2. Connected accounts: one row per (provider, account). `ProviderId`,
--    `AuthMethod` and `ConnectionStatus` mirror the CHECK lists.
CREATE TABLE integration_connections (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    provider TEXT NOT NULL CHECK (provider IN (
        'slack', 'notion', 'linear', 'todoist', 'asana', 'jira', 'monday',
        'clickup', 'trello', 'hubspot', 'attio', 'salesforce', 'affinity',
        'zoho_crm', 'google_docs', 'gmail', 'outlook_mail', 'sandbox'
    )),
    -- The provider's stable ID for the account or workspace (Slack team ID,
    -- Notion workspace ID, ...), so reconnecting updates the same row.
    account_id TEXT NOT NULL CHECK (length(account_id) BETWEEN 1 AND 200),
    -- What the user recognises: "Acme · ana@acme.com".
    account_label TEXT NOT NULL CHECK (length(account_label) BETWEEN 1 AND 200),
    auth_method TEXT NOT NULL CHECK (auth_method IN ('oauth', 'token')),
    status TEXT NOT NULL CHECK (status IN ('connected', 'needs_reconnect')),
    -- Where this account's API lives when the provider decides it per
    -- account (Zoho's `api_domain`, Salesforce's instance, Jira's cloud);
    -- NULL for providers with one fixed API host.
    api_base_url TEXT CHECK (api_base_url IS NULL OR api_base_url LIKE 'https://%'),
    -- When the stored access token expires; NULL when it does not.
    token_expires_at TEXT,
    connected_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- The last request that succeeded with this connection.
    last_used_at TEXT,
    -- The last failure, user-safe (`AppError`): all three or none.
    last_error_code TEXT CHECK (last_error_code IN (
        'not_found', 'invalid', 'conflict', 'unavailable', 'permission_denied',
        'network', 'llm', 'cancelled', 'internal'
    )),
    last_error_message TEXT CHECK (last_error_message IS NULL OR length(last_error_message) BETWEEN 1 AND 500),
    last_error_at TEXT,
    CHECK ((last_error_code IS NULL) = (last_error_message IS NULL)),
    CHECK ((last_error_code IS NULL) = (last_error_at IS NULL)),
    UNIQUE (provider, account_id)
) STRICT;

-- The Integrations page lists connections by provider, oldest first; the
-- UNIQUE index above serves `provider = ?` lookups.
CREATE INDEX idx_integration_connections_list ON integration_connections (provider, connected_at, id);

-- 3. The scopes each connection was granted, as the provider reported them:
--    one row per scope (no JSON list), shown to the user on the page.
CREATE TABLE integration_connection_scopes (
    connection_id TEXT NOT NULL REFERENCES integration_connections (id) ON DELETE CASCADE,
    scope TEXT NOT NULL CHECK (length(scope) BETWEEN 1 AND 300),
    PRIMARY KEY (connection_id, scope)
) STRICT, WITHOUT ROWID;

-- 4. The delivery log: one row per idempotency key. A second send with the
--    same key returns the first result instead of writing twice; a failed
--    row is retried in place (`attempts` counts every try).
CREATE TABLE integration_deliveries (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- Disconnecting keeps the log; the provider column still names it.
    connection_id TEXT REFERENCES integration_connections (id) ON DELETE SET NULL,
    provider TEXT NOT NULL CHECK (provider IN (
        'slack', 'notion', 'linear', 'todoist', 'asana', 'jira', 'monday',
        'clickup', 'trello', 'hubspot', 'attio', 'salesforce', 'affinity',
        'zoho_crm', 'google_docs', 'gmail', 'outlook_mail', 'sandbox'
    )),
    capability TEXT NOT NULL CHECK (capability IN (
        'post_message', 'create_page', 'create_task', 'log_activity'
    )),
    -- The meeting it came from, if any; deleting the meeting keeps the row.
    meeting_id TEXT REFERENCES meetings (id) ON DELETE SET NULL,
    -- Chosen by the caller: `share:<meeting>:<connection>:<target>`, an
    -- automation run's step, ...
    idempotency_key TEXT NOT NULL UNIQUE CHECK (length(idempotency_key) BETWEEN 1 AND 300),
    status TEXT NOT NULL CHECK (status IN ('pending', 'sending', 'succeeded', 'failed')),
    attempts INTEGER NOT NULL CHECK (attempts >= 0),
    -- What the provider created (a Slack message ts, a Notion page ID) and
    -- its link; set once the delivery succeeds.
    external_id TEXT CHECK (external_id IS NULL OR length(external_id) BETWEEN 1 AND 500),
    external_url TEXT CHECK (external_url IS NULL OR external_url LIKE 'http%'),
    -- The last failure, user-safe: both or none.
    error_code TEXT CHECK (error_code IN (
        'not_found', 'invalid', 'conflict', 'unavailable', 'permission_denied',
        'network', 'llm', 'cancelled', 'internal'
    )),
    error_message TEXT CHECK (error_message IS NULL OR length(error_message) BETWEEN 1 AND 500),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((error_code IS NULL) = (error_message IS NULL)),
    CHECK (status <> 'succeeded' OR external_id IS NOT NULL)
) STRICT;

-- The log lists the newest deliveries first.
CREATE INDEX idx_integration_deliveries_recent ON integration_deliveries (created_at DESC, id DESC);
-- Foreign keys: SET NULL on disconnect and on meeting deletion.
CREATE INDEX idx_integration_deliveries_connection ON integration_deliveries (connection_id) WHERE connection_id IS NOT NULL;
CREATE INDEX idx_integration_deliveries_meeting ON integration_deliveries (meeting_id) WHERE meeting_id IS NOT NULL;
