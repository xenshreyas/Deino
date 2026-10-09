-- Every tag on every meeting outside the trash, by meeting then tag: the
-- Meetings page's tag dots and filters. Walks the meeting_tags primary key
-- and probes meetings by ID (asserted in tag_tests.rs).
SELECT
    mt.meeting_id AS "meeting_id: MeetingId",
    mt.tag_id AS "tag_id: TagId",
    mt.applied_by AS "applied_by: TagSource"
FROM meeting_tags AS mt
JOIN meetings AS m ON m.id = mt.meeting_id
WHERE m.deleted_at IS NULL
ORDER BY mt.meeting_id, mt.tag_id
