-- One template's sections (?1), in order. Served by the primary key.
SELECT
    template_id AS "template_id: TemplateId",
    title,
    instructions,
    format AS "format: SectionFormat"
FROM template_sections
WHERE template_id = ?1
ORDER BY position
