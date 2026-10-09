-- Events starting in [?1, ?2), by start time. Served by
-- idx_calendar_events_start_at (asserted in event_repo_tests.rs).
-- Timestamps are RFC 3339 UTC text, whose text order is time order.
SELECT
    id AS "id: CalendarEventId",
    account_id AS "account_id: CalendarAccountId",
    provider_event_id,
    ical_uid,
    series_id AS "series_id: SeriesId",
    title,
    description,
    start_at AS "start_at: DateTime<Utc>",
    end_at AS "end_at: DateTime<Utc>",
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
WHERE start_at >= ?1 AND start_at < ?2
ORDER BY start_at, id
