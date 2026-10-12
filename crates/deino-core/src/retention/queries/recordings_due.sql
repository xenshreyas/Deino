-- The recording files retention deletes: of every finished meeting outside
-- the trash saved at or before ?1 whose recording is still kept and that has a
-- file to delete. One row per meeting, per part merged into it (ORG-07,
-- owned by the absorbed meeting's folders) and per clip (TRN-04, the clip's
-- file in `audio_path`). The meeting's own row comes first.
WITH due (id) AS (
    SELECT m.id
    FROM meetings AS m
    WHERE m.deleted_at IS NULL
        AND m.audio_deleted_at IS NULL
        AND m.status IN ('ready', 'failed')
        AND julianday(m.created_at) <= julianday(?1)
        AND (
            coalesce(m.audio_path, m.audio_mic_path, m.audio_system_path, m.video_path) IS NOT NULL
            OR EXISTS (SELECT 1 FROM clips AS c WHERE c.meeting_id = m.id)
            OR EXISTS (
                SELECT 1 FROM meeting_merge_parts AS p
                WHERE p.meeting_id = m.id
                    AND coalesce(p.audio_path, p.audio_mic_path, p.audio_system_path, p.video_path) IS NOT NULL
            )
        )
)
SELECT
    m.id AS "meeting!: MeetingId",
    m.id AS "owner!: MeetingId",
    m.audio_path AS "audio_path?: String",
    m.audio_mic_path AS "audio_mic_path?: String",
    m.audio_system_path AS "audio_system_path?: String",
    m.video_path AS "video_path?: String"
FROM meetings AS m
WHERE m.id IN (SELECT id FROM due)
UNION ALL
SELECT
    p.meeting_id,
    p.source_meeting_id,
    p.audio_path,
    p.audio_mic_path,
    p.audio_system_path,
    p.video_path
FROM meeting_merge_parts AS p
WHERE p.meeting_id IN (SELECT id FROM due)
UNION ALL
SELECT c.meeting_id, c.meeting_id, c.path, NULL, NULL, NULL
FROM clips AS c
WHERE c.meeting_id IN (SELECT id FROM due)
