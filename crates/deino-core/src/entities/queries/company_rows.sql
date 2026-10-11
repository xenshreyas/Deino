-- The Companies list: every company not merged into another, with how many
-- listed people work there and the meetings outside the trash that any of
-- them attended. Most recently met first, then by name. Computed on read
-- like the People list (ADR 0030).
SELECT
    c.id AS "id!: CompanyId",
    c.name AS "name!",
    c.domain AS "domain: CompanyDomain",
    c.logo_path,
    count(DISTINCT p.id) AS "people_count!: u32",
    count(DISTINCT m.id) AS "meeting_count!: u32",
    max(coalesce(m.started_at, m.created_at)) AS "last_met_at?: DateTime<Utc>"
FROM companies AS c
LEFT JOIN people AS p ON p.company_id = c.id AND p.merged_into IS NULL AND p.is_me = 0
LEFT JOIN meeting_attendees AS a ON a.person_id = p.id
LEFT JOIN meetings AS m
    ON m.id = a.meeting_id AND m.deleted_at IS NULL AND m.status <> 'scheduled'
WHERE c.merged_into IS NULL
GROUP BY c.id
ORDER BY 7 IS NULL, 7 DESC, c.name COLLATE NOCASE, c.id
