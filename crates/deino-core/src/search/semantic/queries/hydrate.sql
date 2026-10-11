-- The chunks with the row IDs in the JSON array ?1, with their meeting's
-- title and date, leaving out trashed meetings (and rows deleted since the
-- in-memory index loaded them).
SELECT
    e.id AS "id!",
    e.meeting_id AS "meeting_id!: MeetingId",
    e.ref_kind AS "kind!: ChunkKind",
    e.ref_id AS "ref_id!",
    e.ts_ms AS "ts_ms: Millis",
    e.end_ms AS "end_ms: Millis",
    e.text AS "text!",
    m.title AS "title!",
    coalesce(m.started_at, m.created_at) AS "date!: DateTime<Utc>"
FROM embeddings AS e
JOIN meetings AS m ON m.id = e.meeting_id
WHERE e.id IN (SELECT value FROM json_each(?1)) AND m.deleted_at IS NULL
