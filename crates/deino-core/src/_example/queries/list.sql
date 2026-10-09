-- Every note, newest first. Queries that tests inspect with EXPLAIN QUERY PLAN
-- live in files like this one, so the test checks the exact text the app runs.
SELECT
    id AS "id: ExampleNoteId",
    text,
    created_at AS "created_at: DateTime<Utc>"
FROM example_notes
ORDER BY created_at DESC, id DESC
