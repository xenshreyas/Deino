-- One API key and its hash, for authentication.
SELECT
    id AS "id: ApiKeyId",
    name,
    scope AS "scope: ApiKeyScope",
    secret_hash,
    created_at AS "created_at: DateTime<Utc>",
    last_used_at AS "last_used_at: DateTime<Utc>",
    revoked_at AS "revoked_at: DateTime<Utc>"
FROM api_keys
WHERE id = ?1
