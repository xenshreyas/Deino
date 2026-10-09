-- Every person not merged into another, by name. Served by the partial
-- index idx_people_listed_name (asserted in repo_tests.rs).
SELECT
    id AS "id: PersonId",
    name,
    email AS "email: Email",
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
WHERE merged_into IS NULL
ORDER BY name COLLATE NOCASE, id
