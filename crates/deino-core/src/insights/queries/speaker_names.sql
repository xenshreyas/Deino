-- The current name of each cited segment's speaker: their person, else the
-- user (Me) for the user's voice, else the speaker's label, else the channel.
-- ?1 is a JSON array of segment IDs.
SELECT
    s.id AS "segment_id!: SegmentId",
    coalesce(
        p.name,
        CASE WHEN sp.is_me = 1 OR (sp.id IS NULL AND s.channel = 'mic')
            THEN coalesce((SELECT name FROM people WHERE is_me = 1 LIMIT 1), 'Me') END,
        sp.label,
        CASE s.channel WHEN 'system' THEN 'Them' ELSE 'Speaker' END
    ) AS "name!: String"
FROM transcript_segments AS s
LEFT JOIN speakers AS sp ON sp.id = s.speaker_id
LEFT JOIN people AS p ON p.id = sp.person_id
WHERE s.id IN (SELECT value FROM json_each(?1))
