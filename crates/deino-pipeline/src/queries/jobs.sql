-- A meeting's jobs (?1). Served by the primary key (meeting_id, stage),
-- asserted in repo_tests.rs; callers order them by the stage registry.
SELECT
    meeting_id AS "meeting_id!: MeetingId",
    stage AS "stage!: StageId",
    status AS "status: JobStatus",
    attempts AS "attempts: u32",
    not_before AS "not_before: DateTime<Utc>",
    message,
    updated_at AS "updated_at: DateTime<Utc>"
FROM jobs
WHERE meeting_id = ?1
ORDER BY stage
