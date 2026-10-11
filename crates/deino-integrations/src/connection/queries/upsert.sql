-- Adds a connection, or refreshes the one for the same account: reconnecting
-- keeps its ID (so its credential key and delivery log stay), takes the new
-- label, method and expiry, and clears any error. Returns the row's ID.
INSERT INTO integration_connections (
    id, provider, account_id, account_label, auth_method, status, api_base_url,
    token_expires_at, connected_at, updated_at
)
VALUES (?1, ?2, ?3, ?4, ?5, 'connected', ?6, ?7, ?8, ?8)
ON CONFLICT (provider, account_id) DO UPDATE SET
    account_label = excluded.account_label,
    auth_method = excluded.auth_method,
    status = 'connected',
    api_base_url = excluded.api_base_url,
    token_expires_at = excluded.token_expires_at,
    updated_at = excluded.updated_at,
    last_error_code = NULL,
    last_error_message = NULL,
    last_error_at = NULL
RETURNING id AS "id: ConnectionId"
