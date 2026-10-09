-- Every calendar account by provider and name, through the UNIQUE
-- (provider, name) index (asserted in account_repo_tests.rs).
SELECT
    id AS "id: CalendarAccountId",
    provider AS "provider: CalendarProvider",
    name,
    created_at AS "created_at: DateTime<Utc>"
FROM calendar_accounts
ORDER BY provider, name
