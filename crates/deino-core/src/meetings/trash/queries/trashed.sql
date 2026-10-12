-- The Trash view: meetings in the trash, most recently trashed first.
-- Served by the partial index idx_meetings_trashed_deleted_at.
SELECT
    id AS "id!: MeetingId",
    title,
    created_at AS "created_at: DateTime<Utc>",
    deleted_at AS "deleted_at!: DateTime<Utc>"
FROM meetings
WHERE deleted_at IS NOT NULL
ORDER BY deleted_at DESC, id DESC
