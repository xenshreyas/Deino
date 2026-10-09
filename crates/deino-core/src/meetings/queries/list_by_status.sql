-- Meetings outside the trash in one status (?1), newest first. Served by
-- idx_meetings_status_created_at (asserted in lifecycle_repo_tests.rs).
SELECT
    id AS "id!: MeetingId",
    title,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM meetings
WHERE status = ?1 AND deleted_at IS NULL
ORDER BY created_at DESC, id DESC
