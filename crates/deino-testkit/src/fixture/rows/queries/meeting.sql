-- A fixture meeting: ?1 id, ?2 title, ?3 when recording stopped (also its
-- created_at, as the legacy path saved meetings then), ?4 status, ?5 source,
-- ?6 started_at, ?7 duration_s, ?8 calendar event, ?9 series. Re-seeding
-- restores every column and takes the meeting out of the trash.
INSERT INTO meetings (
    id, title, created_at, updated_at, status, source, started_at, ended_at, duration_s,
    calendar_event_id, series_id
)
VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?3, ?7, ?8, ?9)
ON CONFLICT (id) DO UPDATE SET
    title = excluded.title,
    created_at = excluded.created_at,
    updated_at = excluded.updated_at,
    status = excluded.status,
    source = excluded.source,
    started_at = excluded.started_at,
    ended_at = excluded.ended_at,
    duration_s = excluded.duration_s,
    calendar_event_id = excluded.calendar_event_id,
    series_id = excluded.series_id,
    deleted_at = NULL
