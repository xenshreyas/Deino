-- Every auto-add rule, folder by folder in the order each folder's rules
-- were added: what the `auto_folder` stage evaluates. Read through
-- idx_folder_rules_folder_id (asserted in rule_repo_tests.rs).
SELECT
    id AS "id: FolderRuleId",
    folder_id AS "folder_id: FolderId",
    kind AS "kind: FolderRuleKind",
    series_id AS "series_id: SeriesId",
    tag_id AS "tag_id: TagId",
    domain,
    created_at AS "created_at: DateTime<Utc>",
    NULL AS "series_title?: String"
FROM folder_rules
ORDER BY folder_id, created_at, id
