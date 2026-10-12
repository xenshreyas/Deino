-- What the summary of a person (?1) reads: their latest ?2 meetings outside
-- the trash that already happened, most recent first, each with its title,
-- the Markdown of its newest enhanced notes (NOT-02, the same read as the
-- auto-tagger's digest, through the enhanced_notes primary key) and the
-- user's own notes.
SELECT
    m.id AS "meeting_id!: MeetingId",
    m.title,
    coalesce(m.started_at, m.created_at) AS "at!: DateTime<Utc>",
    (
        SELECT e.markdown
        FROM enhanced_notes AS e
        WHERE e.meeting_id = m.id
        ORDER BY e.version DESC
        LIMIT 1
    ) AS "summary?: String",
    (SELECT n.notes_markdown FROM meeting_notes AS n WHERE n.meeting_id = m.id) AS "notes?: String"
FROM meeting_attendees AS a
JOIN meetings AS m ON m.id = a.meeting_id
WHERE a.person_id = ?1 AND m.deleted_at IS NULL AND m.status <> 'scheduled'
ORDER BY 3 DESC, m.id DESC
LIMIT ?2
