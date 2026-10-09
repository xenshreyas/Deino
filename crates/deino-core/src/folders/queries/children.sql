-- The folders directly under ?1 (NULL: the top level), by name. Served by
-- idx_folders_parent_id (asserted in folder_tests.rs).
SELECT
    id AS "id: FolderId",
    parent_id AS "parent_id: FolderId",
    name,
    icon,
    description,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM folders
WHERE parent_id IS ?1
ORDER BY name COLLATE NOCASE
