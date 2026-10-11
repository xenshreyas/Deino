-- A meeting outside the trash: its title, and the template its notes were
-- written with (the newest enhanced notes' own), else the one chosen for it.
SELECT
    m.title,
    coalesce(
        (SELECT e.template_id FROM enhanced_notes AS e
         WHERE e.meeting_id = m.id ORDER BY e.version DESC LIMIT 1),
        m.template_id
    ) AS "template_id: TemplateId"
FROM meetings AS m
WHERE m.id = ?1 AND m.deleted_at IS NULL
