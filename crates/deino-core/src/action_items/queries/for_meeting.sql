-- A meeting's (?1) items in the order they were created. Served by
-- idx_action_items_meeting_id (asserted in action_item_tests.rs).
SELECT
    id AS "id: ActionItemId",
    meeting_id AS "meeting_id: MeetingId",
    title,
    description,
    assignee_person_id AS "assignee_person_id: PersonId",
    due_at AS "due_at: DateTime<Utc>",
    status AS "status: ActionItemStatus",
    priority AS "priority: ActionItemPriority",
    source_segment_id AS "source_segment_id: SegmentId",
    completed_at AS "completed_at: DateTime<Utc>",
    created_by AS "created_by: ActionItemOrigin",
    created_at AS "created_at: DateTime<Utc>",
    updated_at AS "updated_at: DateTime<Utc>"
FROM action_items
WHERE meeting_id = ?1
ORDER BY created_at, id
