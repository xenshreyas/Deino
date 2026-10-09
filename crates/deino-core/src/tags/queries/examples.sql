-- Up to ?3 meetings outside the trash carrying a tag (?1), newest first,
-- other than the meeting being tagged (?2): the auto-tagger's examples of
-- what the tag means. Filtered through idx_meeting_tags_tag_id (asserted in
-- auto_tag_repo_tests.rs).
SELECT
    m.id AS "meeting_id!: MeetingId",
    m.title,
    (
        SELECT CASE WHEN json_valid(sp.result) THEN json_extract(sp.result, '$.markdown') END
        FROM summary_processes AS sp
        WHERE sp.meeting_id = m.id AND lower(sp.status) = 'completed'
    ) AS "summary?: String"
FROM meeting_tags AS mt
JOIN meetings AS m ON m.id = mt.meeting_id
WHERE mt.tag_id = ?1 AND mt.meeting_id <> ?2 AND m.deleted_at IS NULL
ORDER BY m.created_at DESC, m.id DESC
LIMIT ?3
