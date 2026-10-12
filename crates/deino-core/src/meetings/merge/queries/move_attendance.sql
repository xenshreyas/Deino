-- Moves meeting ?2's attendance into ?1. Someone at both keeps the stronger
-- role (organizer > invitee > speaker_only) and the first known RSVP; the
-- person merge's rule (ADR 0036, `entities/merge_repo.rs`) generalised.
INSERT INTO meeting_attendees (meeting_id, person_id, role, rsvp)
SELECT ?1, person_id, role, rsvp FROM meeting_attendees WHERE meeting_id = ?2
ON CONFLICT (meeting_id, person_id) DO UPDATE SET
    role = CASE
        WHEN (CASE excluded.role WHEN 'organizer' THEN 3 WHEN 'invitee' THEN 2 ELSE 1 END)
            > (CASE meeting_attendees.role WHEN 'organizer' THEN 3 WHEN 'invitee' THEN 2 ELSE 1 END)
        THEN excluded.role
        ELSE meeting_attendees.role
    END,
    rsvp = coalesce(meeting_attendees.rsvp, excluded.rsvp)
