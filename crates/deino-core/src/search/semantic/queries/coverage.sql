-- How much of the meetings with something to embed the model ?1 covers:
-- meetings outside the trash with transcript, notes or a summary, and how
-- many of them have chunks embedded by ?1 (served by
-- idx_embeddings_model_meeting_id without reading the vectors).
SELECT
    count(*) AS "total!: u32",
    coalesce(sum(EXISTS (
        SELECT 1 FROM embeddings AS e WHERE e.model = ?1 AND e.meeting_id = m.id
    )), 0) AS "indexed!: u32"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND EXISTS (
        SELECT 1 FROM search_documents AS d
        WHERE d.meeting_id = m.id AND d.kind IN ('segment', 'note', 'summary')
    )
