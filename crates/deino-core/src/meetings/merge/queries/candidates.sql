-- The meetings in ?1 (a JSON array of IDs) as a merge sees them. `length`
-- is the recording or the transcript, whichever runs longer, in
-- milliseconds; `parts` counts meetings already merged into each.
SELECT
    m.id AS "id!: MeetingId",
    m.title AS "title!",
    coalesce(m.started_at, m.created_at) AS "held_at!: DateTime<Utc>",
    m.status AS "status!: MeetingStatus",
    m.deleted_at IS NOT NULL AS "trashed!: bool",
    max(
        coalesce(m.duration_s, 0) * 1000,
        coalesce((SELECT max(s.end_ms) FROM transcript_segments AS s WHERE s.meeting_id = m.id), 0)
    ) AS "length!: i64",
    (SELECT count(*) FROM meeting_merge_parts AS p WHERE p.meeting_id = m.id) AS "parts!: i64",
    m.audio_path,
    m.audio_mic_path,
    m.audio_system_path,
    m.video_path
FROM meetings AS m
WHERE m.id IN (SELECT value FROM json_each(?1))
