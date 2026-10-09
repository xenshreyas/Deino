-- Every folder, grouped by parent (top level first) and by name within a
-- parent. Served by idx_folders_parent_id (asserted in folder_tests.rs).
SELECT
    id AS "id: FolderId",
    parent_id AS "parent_id: FolderId",
    name,
    icon,
    description,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM folders
ORDER BY parent_id, name COLLATE NOCASE
