-- The recording files of the meetings in ?1 (a JSON array of IDs), and of
-- every part merged into them, each with the meeting whose folders hold it.
SELECT
    m.id AS "meeting!: MeetingId",
    m.id AS "owner!: MeetingId",
    m.audio_path,
    m.audio_mic_path,
    m.audio_system_path,
    m.video_path
FROM meetings AS m
WHERE m.id IN (SELECT value FROM json_each(?1))
UNION ALL
SELECT
    p.meeting_id,
    p.source_meeting_id,
    p.audio_path,
    p.audio_mic_path,
    p.audio_system_path,
    p.video_path
FROM meeting_merge_parts AS p
WHERE p.meeting_id IN (SELECT value FROM json_each(?1))
