-- Every insight definition, oldest first (`idx_insight_defs_created`).
SELECT
    id AS "id: InsightId",
    name,
    description,
    fields_json AS "fields: Json<Vec<InsightField>>",
    max_answers,
    apply_mode AS "apply_mode: InsightApplyMode",
    conditions_json AS "conditions: Json<ConditionGroups>",
    revision,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM insight_defs
ORDER BY created_at, id
