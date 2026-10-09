-- Every tag by name, ignoring case (the column's collation). Served by the
-- UNIQUE name index (asserted in tag_tests.rs).
SELECT
    id AS "id: TagId",
    name,
    color AS "color: PaletteColor",
    description,
    auto_apply AS "auto_apply: bool",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM tags
ORDER BY name
