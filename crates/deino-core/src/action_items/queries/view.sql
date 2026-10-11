-- One action item (?1) with its list row's meeting, assignee and evidence
-- moment, as the `list_by_*.sql` queries read them (by primary key).
SELECT
    a.id AS "id: ActionItemId",
    a.meeting_id AS "meeting_id: MeetingId",
    a.title,
    a.description,
    a.assignee_person_id AS "assignee_person_id: PersonId",
    a.due_at AS "due_at: DateTime<Utc>",
    a.status AS "status: ActionItemStatus",
    a.priority AS "priority: ActionItemPriority",
    a.source_segment_id AS "source_segment_id: SegmentId",
    a.completed_at AS "completed_at: DateTime<Utc>",
    a.created_by AS "created_by: ActionItemOrigin",
    a.created_at AS "created_at: DateTime<Utc>",
    a.updated_at AS "updated_at: DateTime<Utc>",
    m.title AS "meeting_title?",
    coalesce(m.started_at, m.created_at) AS "meeting_held_at?: DateTime<Utc>",
    p.name AS "assignee_name?",
    coalesce(p.is_me, 0) AS "assignee_is_me!: bool",
    s.start_ms AS "source_start?: Millis"
FROM action_items AS a
LEFT JOIN meetings AS m ON m.id = a.meeting_id
LEFT JOIN people AS p ON p.id = a.assignee_person_id
LEFT JOIN transcript_segments AS s ON s.id = a.source_segment_id
WHERE a.id = ?1
