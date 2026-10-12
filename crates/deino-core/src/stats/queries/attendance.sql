-- Who attended the counted meetings that started at or after ?1 (every
-- meeting when NULL), one row per meeting and person. Declined invitees did
-- not take part and are left out. Attendance is read through the
-- meeting_attendees primary key and each person through theirs.
SELECT
    a.meeting_id AS "meeting_id!: MeetingId",
    p.id AS "person_id!: PersonId",
    p.name AS "name!",
    p.email AS "email: Email",
    p.is_me AS "is_me!: bool"
FROM stats_meetings AS m
JOIN meeting_attendees AS a ON a.meeting_id = m.id
JOIN people AS p ON p.id = a.person_id
WHERE (?1 IS NULL OR julianday(m.at) >= julianday(?1))
    AND (a.rsvp IS NULL OR a.rsvp <> 'declined')
