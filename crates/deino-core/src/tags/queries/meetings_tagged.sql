-- The meetings outside the trash carrying a tag (?1), newest first.
-- Filtered through idx_meeting_tags_tag_id and joined on the meetings
-- primary key (asserted in tag_tests.rs).
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    m.created_at AS "created_at: DateTime<Utc>",
    m.updated_at AS "updated_at: DateTime<Utc>"
FROM meeting_tags AS mt
JOIN meetings AS m ON m.id = mt.meeting_id
WHERE mt.tag_id = ?1 AND m.deleted_at IS NULL
ORDER BY m.created_at DESC, m.id DESC
