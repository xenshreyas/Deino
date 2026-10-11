-- A meeting's (?1) speakers with the person each is, how long each spoke
-- (its visible segments; idx_transcript_segments_speaker_id) and any pending
-- suggestion; most speech first. Speakers come from the UNIQUE
-- (meeting_id, label) index.
SELECT
    s.id AS "id: SpeakerId",
    s.meeting_id AS "meeting_id: MeetingId",
    s.label,
    s.person_id AS "person_id: PersonId",
    s.is_me AS "is_me: bool",
    s.color AS "color: PaletteColor",
    p.name AS "person_name?",
    g.person_id AS "suggested_id?: PersonId",
    gp.name AS "suggested_name?",
    g.score AS "suggested_score?",
    (
        SELECT coalesce(sum(t.end_ms - t.start_ms), 0)
        FROM transcript_segments AS t
        WHERE t.speaker_id = s.id AND t.is_deleted = 0
    ) AS "speech!: Millis"
FROM speakers AS s
LEFT JOIN people AS p ON p.id = s.person_id
LEFT JOIN speaker_suggestions AS g ON g.speaker_id = s.id
LEFT JOIN people AS gp ON gp.id = g.person_id
WHERE s.meeting_id = ?1
ORDER BY 11 DESC, s.label
