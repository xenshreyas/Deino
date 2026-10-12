-- Meetings trashed before ?1, oldest first: what the 30-day purge deletes.
-- `deleted_at` is written only by the trash, in the same encoding as ?1, so
-- the text comparison is a range search on idx_meetings_trashed_deleted_at.
SELECT id AS "id!: MeetingId"
FROM meetings
WHERE deleted_at IS NOT NULL AND deleted_at < ?1
ORDER BY deleted_at, id
