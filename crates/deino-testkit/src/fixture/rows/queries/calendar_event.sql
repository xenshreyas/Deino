-- A fixture's calendar event, upserted by its provider ID like a sync:
-- ?1 id, ?2 the fixture account's name, ?3 provider ID, ?4 iCalendar UID,
-- ?5 series, ?6 title, ?7 description, ?8 start, ?9 end, ?10 link,
-- ?11 platform, ?12 organizer, ?13 attendees JSON, ?14 my RSVP,
-- ?15 is external, ?16 when it was synced, ?17 the fixture calendar.
INSERT INTO calendar_events (
    id, account_id, calendar_id, provider_event_id, ical_uid, series_id, title, description,
    start_at, end_at, conference_url, platform, organizer_email, attendees_json, my_rsvp,
    is_external, created_at, updated_at
)
VALUES (
    ?1, (SELECT id FROM calendar_accounts WHERE provider = 'apple' AND name = ?2), ?17, ?3, ?4,
    ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16
)
ON CONFLICT (account_id, provider_event_id) DO UPDATE SET
    calendar_id = excluded.calendar_id,
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
