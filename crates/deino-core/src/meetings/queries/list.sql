-- Every meeting, newest first. Served by idx_meetings_created_at_id
-- (asserted in repo.rs tests), so the list never sorts in a temporary B-tree.
SELECT
    id AS "id!: MeetingId",
    title,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM meetings
ORDER BY created_at DESC, id DESC
