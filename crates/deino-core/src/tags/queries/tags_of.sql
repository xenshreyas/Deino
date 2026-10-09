-- The tags on a meeting (?1), by name, with who applied them. Filtered
-- through the meeting_tags primary key (asserted in tag_tests.rs).
SELECT
    t.id AS "id: TagId",
    t.name,
    t.color AS "color: PaletteColor",
    t.description,
    t.auto_apply AS "auto_apply: bool",
    t.created_at AS "created_at: DateTime<Utc>",
    t.updated_at AS "updated_at: DateTime<Utc>",
    mt.applied_by AS "applied_by: TagSource",
    mt.applied_at AS "applied_at: DateTime<Utc>"
FROM meeting_tags AS mt
JOIN tags AS t ON t.id = mt.tag_id
WHERE mt.meeting_id = ?1
ORDER BY t.name
