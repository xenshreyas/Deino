-- One person by ID (?1).
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
WHERE id = ?1
