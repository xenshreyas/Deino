-- Every company not merged into another, by name, case-insensitively.
-- Served by the partial index idx_companies_listed_name (asserted in
-- repo_tests.rs).
SELECT
    id AS "id: CompanyId",
    name,
    domain AS "domain: CompanyDomain",
    logo_path,
    notes_md,
    summary_md,
    summary_updated_at AS "summary_updated_at: DateTime<Utc>",
    logo_checked_at AS "logo_checked_at: DateTime<Utc>",
    merged_into AS "merged_into: CompanyId",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM companies
WHERE merged_into IS NULL
ORDER BY name COLLATE NOCASE, id
