-- The user's own person. Served by the unique partial index idx_people_is_me
-- (asserted in repo_tests.rs).
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
WHERE is_me = 1
