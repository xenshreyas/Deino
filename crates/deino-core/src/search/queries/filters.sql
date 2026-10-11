-- `@` filter candidates: people (not merged), companies, tags, folders and
-- meetings outside the trash, in that order, then by name.
--
-- ?1  a filter kind to keep, or NULL for every kind
-- ?2  a LIKE pattern on the name (`prefix%`, or the exact name); LIKE
--     ignores ASCII case
-- ?3  a LIKE pattern for a later word of the name (`% prefix%`), or NULL
-- ?4  the most candidates
--
-- Small tables; the meetings scan is bounded by the user's meeting count.
WITH entries (kind, id, label, rank) AS (
    SELECT 'person', id, name, 1 FROM people WHERE merged_into IS NULL
    UNION ALL
    SELECT 'company', id, name, 2 FROM companies
    UNION ALL
    SELECT 'tag', id, name, 3 FROM tags
    UNION ALL
    SELECT 'folder', id, name, 4 FROM folders
    UNION ALL
    SELECT 'meeting', id, title, 5 FROM meetings WHERE deleted_at IS NULL
)
SELECT
    kind AS "kind!: FilterKind",
    id AS "id!",
    label AS "label!"
FROM entries
WHERE (?1 IS NULL OR kind = ?1)
    AND (label LIKE ?2 ESCAPE '\' OR label LIKE ?3 ESCAPE '\')
ORDER BY rank, label COLLATE NOCASE, id
LIMIT ?4
