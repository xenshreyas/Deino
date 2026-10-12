-- The avatars of a page's meetings (?1, a JSON array of meeting IDs): each
-- attendee, the user last, then by name. One lookup per meeting through
-- the meeting_attendees primary key.
SELECT
    a.meeting_id AS "meeting_id!: MeetingId",
    p.id AS "person_id!: PersonId",
    p.name AS "name!",
    p.avatar_path,
    p.is_me AS "is_me!: bool"
FROM json_each(?1) AS page
JOIN meeting_attendees AS a ON a.meeting_id = page.value
JOIN people AS p ON p.id = a.person_id
ORDER BY a.meeting_id, p.is_me, p.name COLLATE NOCASE, p.id
