-- The "Unfiled" smart folder: meetings outside the trash that sit in no
-- folder, newest first. Walks idx_meetings_live_created_at_id backwards and
-- probes the meeting_folders primary key per meeting (asserted in
-- folder_tests.rs), so it never sorts.
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    m.created_at AS "created_at: DateTime<Utc>",
    m.updated_at AS "updated_at: DateTime<Utc>"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND NOT EXISTS (SELECT 1 FROM meeting_folders AS mf WHERE mf.meeting_id = m.id)
ORDER BY m.created_at DESC, m.id DESC
