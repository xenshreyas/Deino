-- Unmerged people whose name, a later word of their name, or email starts
-- with ?1 (a LIKE pattern prefix, wildcards already escaped with `\`), by
-- name, at most ?2. Walks idx_people_listed_name in order and stops at the
-- limit, so it stays fast without a full-text index (ORG-08 adds one).
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
    AND (
        name LIKE ?1 || '%' ESCAPE '\'
        OR name LIKE '% ' || ?1 || '%' ESCAPE '\'
        OR email LIKE ?1 || '%' ESCAPE '\'
    )
ORDER BY name COLLATE NOCASE, id
LIMIT ?2
