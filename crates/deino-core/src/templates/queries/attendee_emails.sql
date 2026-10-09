-- A meeting's attendees' addresses (?1); NULL for people without one.
SELECT people.email AS "email: Email"
FROM meeting_attendees
JOIN people ON people.id = meeting_attendees.person_id
WHERE meeting_attendees.meeting_id = ?1
