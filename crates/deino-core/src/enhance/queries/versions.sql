-- A meeting's (?1) versions, newest first, with their template's name: the
-- version history menu. The primary key serves the filter and the order.
SELECT
    e.version AS "version!: u32",
    e.created_at AS "created_at!: DateTime<Utc>",
    e.updated_at AS "updated_at!: DateTime<Utc>",
    e.template_id AS "template_id: TemplateId",
    t.name AS "template_name?: String",
    e.model,
    e.is_user_edited AS "is_user_edited!: bool"
FROM enhanced_notes AS e
LEFT JOIN templates AS t ON t.id = e.template_id
WHERE e.meeting_id = ?1
ORDER BY e.version DESC
