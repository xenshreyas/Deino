-- The meetings outside the trash in a folder (?1), newest first. Filtered
-- through idx_meeting_folders_folder_id and joined on the meetings primary
-- key (asserted in folder_tests.rs); one folder is small enough to sort.
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    m.created_at AS "created_at: DateTime<Utc>",
    m.updated_at AS "updated_at: DateTime<Utc>"
FROM meeting_folders AS mf
JOIN meetings AS m ON m.id = mf.meeting_id
WHERE mf.folder_id = ?1 AND m.deleted_at IS NULL
ORDER BY m.created_at DESC, m.id DESC
