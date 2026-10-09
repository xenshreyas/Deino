-- Every company by name, case-insensitively. Served by idx_companies_name
-- (asserted in repo_tests.rs).
SELECT
    id AS "id: CompanyId",
    name,
    domain AS "domain: CompanyDomain",
    logo_path,
    notes_md,
    summary_md,
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM companies
ORDER BY name COLLATE NOCASE, id
