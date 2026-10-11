-- Every calendar, by account and title (case-insensitive). A handful of
-- rows per account, so the sort stays in memory.
SELECT
    id AS "id: CalendarId",
    account_id AS "account_id: CalendarAccountId",
    provider_calendar_id,
    title,
    color AS "color: CalendarColor",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM calendars
ORDER BY account_id, title COLLATE NOCASE, id
