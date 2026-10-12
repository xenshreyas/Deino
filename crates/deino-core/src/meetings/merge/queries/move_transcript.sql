-- Moves meeting ?2's transcript into ?1, shifted by ?3 milliseconds (each
-- word's timing too). A line whose speaker stayed behind (the user, or a
-- person ?1 already has a speaker for) joins ?1's speaker; the others'
-- speakers have already moved. Search triggers refresh each line.
UPDATE transcript_segments AS g
SET
    meeting_id = ?1,
    start_ms = g.start_ms + ?3,
    end_ms = g.end_ms + ?3,
    speaker_id = coalesce(
        (
            SELECT t.id
            FROM speakers AS s
            JOIN speakers AS t ON t.meeting_id = ?1
                AND ((t.is_me = 1 AND s.is_me = 1) OR t.person_id = s.person_id)
            WHERE s.id = g.speaker_id AND s.meeting_id = ?2
            ORDER BY t.id
            LIMIT 1
        ),
        g.speaker_id
    ),
    words_json = (
        SELECT coalesce(
            json_group_array(
                json_set(
                    word.value,
                    '$.start', json_extract(word.value, '$.start') + ?3,
                    '$.end', json_extract(word.value, '$.end') + ?3
                )
            ),
            '[]'
        )
        FROM json_each(g.words_json) AS word
    )
WHERE g.meeting_id = ?2
