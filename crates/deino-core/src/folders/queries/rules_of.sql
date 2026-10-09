-- A folder's (?1) auto-add rules in the order they were added, each series
-- rule with the title of its series' latest meeting outside the trash.
-- Served by idx_folder_rules_folder_id, and the title by
-- idx_meetings_series_id (asserted in rule_repo_tests.rs).
SELECT
    r.id AS "id: FolderRuleId",
    r.folder_id AS "folder_id: FolderId",
    r.kind AS "kind: FolderRuleKind",
    r.series_id AS "series_id: SeriesId",
    r.tag_id AS "tag_id: TagId",
    r.domain,
    r.created_at AS "created_at: DateTime<Utc>",
    (
        SELECT m.title FROM meetings AS m
        WHERE m.series_id = r.series_id AND m.deleted_at IS NULL
        ORDER BY m.created_at DESC, m.id DESC
        LIMIT 1
    ) AS "series_title?: String"
FROM folder_rules AS r
WHERE r.folder_id = ?1
ORDER BY r.created_at, r.id
