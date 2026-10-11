-- The People list: every person not merged into another and not the user,
-- or only those at one company (?1, NULL for everyone), with their company
-- and their meetings outside the trash that already happened. Most recently
-- met first, then by name.
--
-- The aggregates are computed here rather than stored (ADR 0030): each
-- person's attendance is read through idx_meeting_attendees_person_id and
-- each meeting through its primary key (asserted in reads_tests.rs).
SELECT
    p.id AS "id!: PersonId",
    p.name AS "name!",
    p.email AS "email: Email",
    p.title,
    p.company_id AS "company_id: CompanyId",
    c.name AS "company_name?: String",
    c.logo_path AS "company_logo_path?: String",
    max(coalesce(m.started_at, m.created_at)) AS "last_met_at?: DateTime<Utc>",
    count(m.id) AS "meeting_count!: u32"
FROM people AS p
LEFT JOIN companies AS c ON c.id = p.company_id
LEFT JOIN meeting_attendees AS a ON a.person_id = p.id
LEFT JOIN meetings AS m
    ON m.id = a.meeting_id AND m.deleted_at IS NULL AND m.status <> 'scheduled'
WHERE p.merged_into IS NULL AND p.is_me = 0 AND (?1 IS NULL OR p.company_id = ?1)
GROUP BY p.id
ORDER BY 8 IS NULL, 8 DESC, p.name COLLATE NOCASE, p.id
