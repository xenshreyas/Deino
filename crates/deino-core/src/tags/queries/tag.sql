-- One tag by ID (?1).
SELECT
    id AS "id: TagId",
    name,
    color AS "color: PaletteColor",
    description,
    auto_apply AS "auto_apply: bool",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM tags
WHERE id = ?1
