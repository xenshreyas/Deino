-- ORG-01: the calendars inside each calendar account, and the event columns
-- calendar sync needs (FND-07 left both for this card, PARITY_PLAN §7.2).
--
-- A calendar account (an iCloud, Google or Exchange account in macOS
-- Calendar, or a directly connected Google or Microsoft account) holds
-- several calendars ("Work", "Family", "US Holidays"); each event belongs to
-- exactly one. Which calendars the user sees is a setting
-- (`Settings::calendar.hiddenCalendars`), not a column, so hiding one never
-- rewrites rows.

CREATE TABLE calendars (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    account_id TEXT NOT NULL REFERENCES calendar_accounts (id) ON DELETE CASCADE,
    -- The provider's ID for the calendar (EventKit's calendarIdentifier,
    -- Google's calendar ID); unique per account, so syncs upsert.
    provider_calendar_id TEXT NOT NULL CHECK (length(provider_calendar_id) > 0),
    title TEXT NOT NULL DEFAULT '',
    -- The calendar's colour as the provider shows it, "#RRGGBB" in upper case.
    color TEXT CHECK (color IS NULL OR (length(color) = 7 AND color GLOB '#[0-9A-F][0-9A-F][0-9A-F][0-9A-F][0-9A-F][0-9A-F]')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Also the index for the account foreign key (its leading column) and for
    -- Settings → Calendars, which lists calendars by account.
    UNIQUE (account_id, provider_calendar_id)
) STRICT;

-- The calendar an event was synced from. NULL only for rows written before
-- ORG-01 (and the fixture seeder's), which belong to no known calendar.
-- Deleting a calendar deletes its events; a meeting recorded for one keeps
-- its series and loses the link (meetings.calendar_event_id is SET NULL).
ALTER TABLE calendar_events ADD COLUMN calendar_id TEXT REFERENCES calendars (id) ON DELETE CASCADE;

-- An all-day event spans whole days in the user's time zone; `start_at` and
-- `end_at` then hold the UTC instants of its local midnights.
ALTER TABLE calendar_events ADD COLUMN is_all_day INTEGER NOT NULL DEFAULT 0 CHECK (is_all_day IN (0, 1));

CREATE INDEX idx_calendar_events_calendar_id ON calendar_events (calendar_id) WHERE calendar_id IS NOT NULL;

-- A sync replaces an account's events inside its window: it reads the stored
-- ones by account and start time.
CREATE INDEX idx_calendar_events_account_start ON calendar_events (account_id, start_at);
