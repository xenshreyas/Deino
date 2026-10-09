-- Jobs left running, which at startup means a crash interrupted them.
-- Served by idx_jobs_status_meeting_id (asserted in repo_tests.rs).
SELECT
    meeting_id AS "meeting_id!: MeetingId",
    stage AS "stage!: StageId",
    status AS "status: JobStatus",
    attempts AS "attempts: u32",
    not_before AS "not_before: DateTime<Utc>",
    message,
    updated_at AS "updated_at: DateTime<Utc>"
FROM jobs
WHERE status = 'running'
ORDER BY meeting_id, stage
