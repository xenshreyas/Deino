-- One event by ID (?1).
SELECT
    id AS "id: CalendarEventId",
    account_id AS "account_id: CalendarAccountId",
    calendar_id AS "calendar_id: CalendarId",
    provider_event_id,
    ical_uid,
    series_id AS "series_id: SeriesId",
    title,
    description,
    start_at AS "start_at: DateTime<Utc>",
    end_at AS "end_at: DateTime<Utc>",
    is_all_day AS "is_all_day: bool",
    conference_url,
    platform AS "platform: ConferencePlatform",
    organizer_email,
    attendees_json AS "attendees: Json<Vec<EventAttendee>>",
    my_rsvp AS "my_rsvp: Rsvp",
    is_external AS "is_external: bool",
    record_override AS "record_override: bool",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM calendar_events
WHERE id = ?1
