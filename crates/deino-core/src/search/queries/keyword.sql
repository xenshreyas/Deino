-- Keyword hits, best first, at most ?3 of each kind.
--
-- ?1  the FTS5 MATCH expression (`fts::match_expression`)
-- ?2  NULL to search everything outside the trash; else a JSON array of
--     meeting IDs, and only those meetings' documents are searched
-- ?3  hits per kind
--
-- `rank` is the table's BM25 (title matches weigh four times body matches,
-- set in the ORG-08 migration). Unfiltered, only the best 1,000 matches are
-- considered, through FTS5's own ORDER BY rank LIMIT: a word in half of all
-- segments would otherwise rank every one of them. A filtered search ranks
-- all its matches, since the best global ones may lie outside the filter.
-- highlight() and snippet(), the expensive part, run only for the rows that
-- survive the per-kind cut.
WITH ranked AS (
    SELECT search_fts.rowid AS doc, search_fts.rank AS rank
    FROM search_fts
    WHERE search_fts MATCH ?1
    ORDER BY search_fts.rank
    LIMIT CASE WHEN ?2 IS NULL THEN 1000 ELSE -1 END
),
kept AS (
    SELECT
        ranked.doc,
        ranked.rank,
        row_number() OVER (PARTITION BY d.kind ORDER BY ranked.rank, ranked.doc) AS place
    FROM ranked
    JOIN search_documents AS d ON d.id = ranked.doc
    LEFT JOIN meetings AS m ON m.id = d.meeting_id
    WHERE CASE
        WHEN ?2 IS NULL THEN d.meeting_id IS NULL OR m.deleted_at IS NULL
        ELSE d.meeting_id IN (SELECT value FROM json_each(?2))
    END
)
SELECT
    d.kind AS "kind!: HitKind",
    d.ref_id AS "ref_id!",
    d.meeting_id AS "meeting_id: MeetingId",
    d.ts_ms AS "ts_ms: Millis",
    CASE
        WHEN d.title = '' THEN coalesce(m.title, '')
        ELSE highlight(search_fts, 3, char(2), char(3))
    END AS "label!: String",
    snippet(search_fts, 4, char(2), char(3), '…', 16) AS "snippet!: String",
    coalesce(m.started_at, m.created_at) AS "date: DateTime<Utc>",
    -kept.rank AS "score!: f64"
FROM search_fts
JOIN kept ON kept.doc = search_fts.rowid
JOIN search_documents AS d ON d.id = kept.doc
LEFT JOIN meetings AS m ON m.id = d.meeting_id
WHERE search_fts MATCH ?1 AND kept.place <= ?3
ORDER BY kept.rank, kept.doc
