-- Every section of every template, in order. Served by the primary key.
SELECT
    template_id AS "template_id: TemplateId",
    title,
    instructions,
    format AS "format: SectionFormat"
FROM template_sections
ORDER BY template_id, position
