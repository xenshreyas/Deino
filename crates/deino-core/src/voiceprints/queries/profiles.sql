-- Settings → People → Voice profiles: every unmerged person's exemplars, by
-- name, best exemplar first, with the meeting each was confirmed in.
SELECT
    p.id AS "person_id: PersonId",
    p.name,
    p.is_me AS "is_me: bool",
    v.id AS "id: VoiceprintId",
    v.source_meeting_id AS "source_meeting_id: MeetingId",
    m.title AS "source_meeting_title?",
    v.quality,
    v.created_at AS "created_at: DateTime<Utc>"
FROM voiceprints AS v
JOIN people AS p ON p.id = v.person_id
LEFT JOIN meetings AS m ON m.id = v.source_meeting_id
WHERE p.merged_into IS NULL
ORDER BY p.name COLLATE NOCASE, p.id, v.quality DESC, v.created_at DESC, v.id
