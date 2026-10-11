-- The delivery with idempotency key ?1 (its UNIQUE index).
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
WHERE idempotency_key = ?1
