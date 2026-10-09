-- One folder by ID (?1).
SELECT
    id AS "id: FolderId",
    parent_id AS "parent_id: FolderId",
    name,
    icon,
    description,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM folders
WHERE id = ?1
