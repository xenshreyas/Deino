-- A person's (?1) items in one status (?2), by due date. Filtered through
-- idx_action_items_assignee_status_due_at (asserted in action_item_tests.rs).
-- Items with a due date come first, soonest first; undated ones last. The
-- `due_at IS NULL` term costs a small sort after the indexed filter, which
-- is cheaper than a sentinel date in the data.
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
WHERE assignee_person_id = ?1 AND status = ?2
ORDER BY due_at IS NULL, due_at, id
