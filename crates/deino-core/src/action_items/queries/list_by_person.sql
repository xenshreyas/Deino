-- The Action Items list, read through one assignee. One of three files with the same
-- columns, parameters and filters, differing only in the leading condition
-- that picks the index (idx_action_items_assignee_status_due_at,
-- asserted in list_repo_tests.rs); `ItemQuery::access_path` chooses the file.
-- Parameters (NULL or false do not filter):
--   ?1, ?2  statuses to include          ?3  assignee
--   ?4      only assigned items           ?5  not this assignee
--   ?6      only unassigned items         ?7  meeting
--   ?8      meeting tag                   ?9  company (assignee's or an attendee's)
--   ?10     folder, with its subfolders   ?11, ?12  due in [from, before)
--   ?13     only undated items            ?14, ?15  held in [from, before)
-- Items of meetings in the trash are hidden; standalone items have no meeting.
-- Dated items come first, soonest first; undated ones last.
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
WHERE a.assignee_person_id = ?3
    AND a.status IN (?1, ?2)
    AND (?3 IS NULL OR a.assignee_person_id = ?3)
    AND (NOT ?4 OR a.assignee_person_id IS NOT NULL)
    AND (?5 IS NULL OR a.assignee_person_id IS NOT ?5)
    AND (NOT ?6 OR a.assignee_person_id IS NULL)
    AND (?7 IS NULL OR a.meeting_id = ?7)
    AND (
        ?8 IS NULL
        OR EXISTS (
            SELECT 1 FROM meeting_tags AS mt WHERE mt.meeting_id = a.meeting_id AND mt.tag_id = ?8
        )
    )
    AND (
        ?9 IS NULL
        OR p.company_id = ?9
        OR EXISTS (
            SELECT 1
            FROM meeting_attendees AS ma
            JOIN people AS colleague ON colleague.id = ma.person_id
            WHERE ma.meeting_id = a.meeting_id AND colleague.company_id = ?9
        )
    )
    AND (
        ?10 IS NULL
        OR a.meeting_id IN (
            SELECT mf.meeting_id
            FROM meeting_folders AS mf
            WHERE mf.folder_id IN (
                WITH RECURSIVE subtree (id) AS (
                    SELECT ?10
                    UNION
                    SELECT f.id FROM folders AS f JOIN subtree ON f.parent_id = subtree.id
                )
                SELECT id FROM subtree
            )
        )
    )
    AND (?11 IS NULL OR a.due_at >= ?11)
    AND (?12 IS NULL OR a.due_at < ?12)
    AND (NOT ?13 OR a.due_at IS NULL)
    AND (?14 IS NULL OR coalesce(m.started_at, m.created_at, a.created_at) >= ?14)
    AND (?15 IS NULL OR coalesce(m.started_at, m.created_at, a.created_at) < ?15)
    AND m.deleted_at IS NULL
ORDER BY a.due_at IS NULL, a.due_at, a.id
