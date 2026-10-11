-- Meetings the backfill embeds: finished ones (ready or failed) outside the
-- trash that have something to embed, newest first. Meetings still
-- recording or processing get their `index` stage instead.
SELECT m.id AS "id!: MeetingId"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND m.status IN ('ready', 'failed')
    AND EXISTS (
        SELECT 1 FROM search_documents AS d
        WHERE d.meeting_id = m.id AND d.kind IN ('segment', 'note', 'summary')
    )
ORDER BY coalesce(m.started_at, m.created_at) DESC, m.id DESC
