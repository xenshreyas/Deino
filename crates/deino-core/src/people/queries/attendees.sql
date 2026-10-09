-- A meeting's (?1) attendees in the order they were first added (rowid).
-- Served by the (meeting_id, person_id) primary key (asserted in
-- attendee_repo_tests.rs).
SELECT
    meeting_id AS "meeting_id: MeetingId",
    person_id AS "person_id: PersonId",
    role AS "role: AttendeeRole",
    rsvp AS "rsvp: Rsvp"
FROM meeting_attendees
WHERE meeting_id = ?1
ORDER BY rowid
