-- A meeting's (?1) title and the Markdown of its newest enhanced notes
-- (NOT-02), for the auto-tagger; no row when the meeting is missing or in
-- the trash.
SELECT
    m.id AS "meeting_id!: MeetingId",
    m.title,
    (
        SELECT e.markdown
        FROM enhanced_notes AS e
        WHERE e.meeting_id = m.id
        ORDER BY e.version DESC
        LIMIT 1
    ) AS "summary?: String"
FROM meetings AS m
WHERE m.id = ?1 AND m.deleted_at IS NULL
