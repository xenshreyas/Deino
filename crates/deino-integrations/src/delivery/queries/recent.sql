-- The newest ?1 deliveries (idx_integration_deliveries_recent).
SELECT
    id AS "id: DeliveryId",
    connection_id AS "connection_id: ConnectionId",
    provider AS "provider: IntegrationProvider",
    capability AS "capability: Capability",
    meeting_id AS "meeting_id: MeetingId",
    idempotency_key,
    status AS "status: DeliveryStatus",
    attempts AS "attempts: u32",
    external_id,
    external_url,
    error_code AS "error_code: ErrorCode",
    error_message,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM integration_deliveries
ORDER BY created_at DESC, id DESC
LIMIT ?1
