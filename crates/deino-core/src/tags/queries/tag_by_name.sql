-- One tag by name (?1), ignoring case, through the UNIQUE name index.
SELECT
    id AS "id: TagId",
    name,
    color AS "color: PaletteColor",
    description,
    auto_apply AS "auto_apply: bool",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM tags
WHERE name = ?1
