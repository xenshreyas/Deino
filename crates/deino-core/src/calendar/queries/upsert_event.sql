-- Stores an event, or refreshes the stored copy with the same account and
-- provider ID. A refresh keeps the stored id, created_at and the user's
-- record_override, and touches the row only when something the provider
-- sent changed: an unchanged re-sync writes nothing, so it raises no
-- DbChanged and returns no row (the caller then looks the ID up).
-- ON CONFLICT DO UPDATE, never REPLACE: the update hook misses REPLACE
-- deletions, and REPLACE would drop the override.
INSERT INTO calendar_events (
    id, account_id, calendar_id, provider_event_id, ical_uid, series_id, title, description,
    start_at, end_at, is_all_day, conference_url, platform, organizer_email, attendees_json,
    my_rsvp, is_external, record_override, created_at, updated_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
ON CONFLICT (account_id, provider_event_id) DO UPDATE SET
    calendar_id = excluded.calendar_id,
    ical_uid = excluded.ical_uid,
    series_id = excluded.series_id,
    title = excluded.title,
    description = excluded.description,
    start_at = excluded.start_at,
    end_at = excluded.end_at,
    is_all_day = excluded.is_all_day,
    conference_url = excluded.conference_url,
    platform = excluded.platform,
    organizer_email = excluded.organizer_email,
    attendees_json = excluded.attendees_json,
    my_rsvp = excluded.my_rsvp,
    is_external = excluded.is_external,
    updated_at = excluded.updated_at
WHERE calendar_events.calendar_id IS NOT excluded.calendar_id
    OR calendar_events.ical_uid IS NOT excluded.ical_uid
    OR calendar_events.series_id IS NOT excluded.series_id
    OR calendar_events.title IS NOT excluded.title
    OR calendar_events.description IS NOT excluded.description
    OR calendar_events.start_at IS NOT excluded.start_at
    OR calendar_events.end_at IS NOT excluded.end_at
    OR calendar_events.is_all_day IS NOT excluded.is_all_day
    OR calendar_events.conference_url IS NOT excluded.conference_url
    OR calendar_events.platform IS NOT excluded.platform
    OR calendar_events.organizer_email IS NOT excluded.organizer_email
    OR calendar_events.attendees_json IS NOT excluded.attendees_json
    OR calendar_events.my_rsvp IS NOT excluded.my_rsvp
    OR calendar_events.is_external IS NOT excluded.is_external
RETURNING id AS "id: CalendarEventId"
