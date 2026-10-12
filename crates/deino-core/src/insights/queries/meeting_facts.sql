-- A meeting outside the trash: its title, the template its notes were
-- written with (the newest enhanced notes' own), else the one chosen for it,
-- how long it recorded and the calendar its event is on (AUT-01).
SELECT
    m.title,
    coalesce(
        (SELECT e.template_id FROM enhanced_notes AS e
         WHERE e.meeting_id = m.id ORDER BY e.version DESC LIMIT 1),
        m.template_id
    ) AS "template_id: TemplateId",
    m.duration_s,
    (SELECT c.calendar_id FROM calendar_events AS c
     WHERE c.id = m.calendar_event_id) AS "calendar_id: CalendarId"
FROM meetings AS m
WHERE m.id = ?1 AND m.deleted_at IS NULL
