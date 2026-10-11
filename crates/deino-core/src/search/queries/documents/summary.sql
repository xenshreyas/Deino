-- Summaries (legacy `summary_processes.result`); ?1 is one meeting's ID, or
-- NULL. The same rule as the `search_summaries_*` triggers; nested CASEs
-- because `json_type` fails on text that is not JSON.
INSERT INTO search_documents (kind, ref_id, meeting_id, body)
SELECT 'summary', meeting_id, meeting_id, markdown
FROM (
    SELECT
        meeting_id,
        CASE WHEN json_valid(result) THEN
            CASE WHEN json_type(result, '$.markdown') = 'text' THEN json_extract(result, '$.markdown') END
        END AS markdown
    FROM summary_processes
    WHERE ?1 IS NULL OR meeting_id = ?1
)
WHERE trim(coalesce(markdown, '')) <> ''
