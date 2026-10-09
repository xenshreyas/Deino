-- Every template, by name. Served by idx_templates_name (asserted in repo_tests.rs).
SELECT
    id AS "id: TemplateId",
    name,
    description,
    meeting_context,
    auto_rules_json AS "auto_rules: Json<Vec<AutoRule>>",
    builtin_version,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM templates
ORDER BY name COLLATE NOCASE, id
