-- Every meeting outside the trash, newest first. Served by the partial index
-- idx_meetings_live_created_at_id (asserted in repo.rs tests), so the list
-- never sorts in a temporary B-tree.
SELECT
    id AS "id!: MeetingId",
    title,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM meetings
WHERE deleted_at IS NULL
ORDER BY created_at DESC, id DESC
