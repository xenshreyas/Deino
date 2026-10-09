-- A meeting's (?1) speakers by label. Served by the UNIQUE (meeting_id, label)
-- index (asserted in speaker_repo_tests.rs).
SELECT
    id AS "id: SpeakerId",
    meeting_id AS "meeting_id: MeetingId",
    label,
    person_id AS "person_id: PersonId",
    is_me AS "is_me: bool",
    color AS "color: PaletteColor"
FROM speakers
WHERE meeting_id = ?1
ORDER BY label
