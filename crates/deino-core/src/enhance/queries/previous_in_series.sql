-- The newest notes of the latest meeting of series ?1 before ?3, other than
-- meeting ?2 and outside the trash: the "previous meeting" a prompt quotes.
-- Served by idx_meetings_series_id and the enhanced_notes primary key.
SELECT
    m.title,
    COALESCE(m.started_at, m.created_at) AS "date!: DateTime<Utc>",
    e.markdown
FROM meetings AS m
JOIN enhanced_notes AS e ON e.meeting_id = m.id
WHERE m.series_id = ?1
    AND m.id <> ?2
    AND m.deleted_at IS NULL
    AND COALESCE(m.started_at, m.created_at) < ?3
ORDER BY COALESCE(m.started_at, m.created_at) DESC, e.version DESC
LIMIT 1
