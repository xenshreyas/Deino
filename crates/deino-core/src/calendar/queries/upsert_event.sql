-- Stores an event, or refreshes the stored copy with the same account and
-- provider ID. A refresh keeps the stored id, created_at and the user's
-- record_override. ON CONFLICT DO UPDATE, never REPLACE: the update hook
-- misses REPLACE deletions, and REPLACE would drop the override.
INSERT INTO calendar_events (
    id, account_id, provider_event_id, ical_uid, series_id, title, description, start_at, end_at,
    conference_url, platform, organizer_email, attendees_json, my_rsvp, is_external, record_override,
    created_at, updated_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
ON CONFLICT (account_id, provider_event_id) DO UPDATE SET
    ical_uid = excluded.ical_uid,
    series_id = excluded.series_id,
    title = excluded.title,
    description = excluded.description,
    start_at = excluded.start_at,
    end_at = excluded.end_at,
    conference_url = excluded.conference_url,
    platform = excluded.platform,
    organizer_email = excluded.organizer_email,
    attendees_json = excluded.attendees_json,
    my_rsvp = excluded.my_rsvp,
    is_external = excluded.is_external,
    updated_at = excluded.updated_at
RETURNING id AS "id: CalendarEventId"
