-- How many meetings outside the trash each folder holds; folders holding
-- none are absent. Groups over idx_meeting_folders_folder_id (asserted in
-- folder_tests.rs).
SELECT
    mf.folder_id AS "folder_id: FolderId",
    count(*) AS "meetings!: i64"
FROM meeting_folders AS mf
JOIN meetings AS m ON m.id = mf.meeting_id
WHERE m.deleted_at IS NULL
GROUP BY mf.folder_id
