-- A company's (?1) people not merged into another, by name. Filtered
-- through idx_people_company_id (asserted in repo_tests.rs); a company has
-- few enough people that sorting them is cheap.
SELECT
    id AS "id: PersonId",
    name,
    email AS "email: EmailAddress",
    company_id AS "company_id: CompanyId",
    title,
    avatar_path,
    notes_md,
    summary_md,
    summary_updated_at AS "summary_updated_at: DateTime<Utc>",
    is_me AS "is_me: bool",
    merged_into AS "merged_into: PersonId",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM people
WHERE company_id = ?1 AND merged_into IS NULL
ORDER BY name COLLATE NOCASE, id
