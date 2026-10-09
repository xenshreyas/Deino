-- One action item by ID (?1).
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
WHERE id = ?1
