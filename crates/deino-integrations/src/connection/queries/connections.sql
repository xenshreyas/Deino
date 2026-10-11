-- Every connection, by provider, oldest first (idx_integration_connections_list).
SELECT
    id AS "id: ConnectionId",
    provider AS "provider: IntegrationProvider",
    account_id,
    account_label,
    auth_method AS "auth_method: AuthMethod",
    status AS "status: ConnectionStatus",
    api_base_url,
    token_expires_at AS "token_expires_at: DateTime<Utc>",
    connected_at AS "connected_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>",
    last_used_at AS "last_used_at: DateTime<Utc>",
    last_error_code AS "last_error_code: ErrorCode",
    last_error_message,
    last_error_at AS "last_error_at: DateTime<Utc>"
FROM integration_connections
ORDER BY provider, connected_at, id
