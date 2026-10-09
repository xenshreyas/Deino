-- The folders a meeting (?1) sits in, by name. Filtered through the
-- meeting_folders primary key (asserted in folder_tests.rs).
SELECT
    f.id AS "id: FolderId",
    f.parent_id AS "parent_id: FolderId",
    f.name,
    f.icon,
    f.description,
    f.created_at AS "created_at: DateTime<Utc>",
    f.updated_at AS "updated_at: DateTime<Utc>"
FROM meeting_folders AS mf
JOIN folders AS f ON f.id = mf.folder_id
WHERE mf.meeting_id = ?1
ORDER BY f.name COLLATE NOCASE, f.id
