-- Meetings with a pending or running job, for startup. Served by
-- idx_jobs_status_meeting_id (asserted in repo_tests.rs).
SELECT DISTINCT meeting_id AS "meeting_id!: MeetingId"
FROM jobs
WHERE status IN ('pending', 'running')
ORDER BY meeting_id
