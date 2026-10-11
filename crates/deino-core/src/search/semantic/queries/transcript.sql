-- A meeting's transcript in time order, with who said each line: the linked
-- person's name, else the speaker's label. Soft-deleted segments are left
-- out. Served by idx_transcript_segments_meeting_id_start_ms.
SELECT
    s.id AS "segment_id!",
    s.start_ms AS "start!: Millis",
    s.end_ms AS "end!: Millis",
    coalesce(p.name, sp.label) AS "speaker: String",
    s.text AS "text!"
FROM transcript_segments AS s
LEFT JOIN speakers AS sp ON sp.id = s.speaker_id
LEFT JOIN people AS p ON p.id = sp.person_id
WHERE s.meeting_id = ?1 AND s.is_deleted = 0 AND trim(s.text) <> ''
ORDER BY s.start_ms, s.id
