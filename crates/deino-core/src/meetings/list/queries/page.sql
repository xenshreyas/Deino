-- The Meetings page: meetings outside the trash that pass every filter,
-- newest first, starting after the cursor. Walks the partial index
-- idx_meetings_live_created_at_id in order (asserted in repo_tests.rs), so
-- a page stops after ?13 matches and never sorts.
--
-- Lists are JSON arrays and an empty one does not filter. Within a list any
-- value matches (OR); the conditions combine with AND.
--
-- ?1, ?2  held from (inclusive) and before (exclusive), UTC; NULL: open
-- ?3      person IDs: one attended, or spoke as a linked speaker
-- ?4      company IDs: someone from one attended
-- ?5      tag IDs: one is on the meeting
-- ?6      folder IDs: the meeting is in one, or in a folder inside one
-- ?7      1: only meetings with action items
-- ?8      'internal' or 'external'; NULL does not filter
-- ?9      the user's own addresses, ?10 their work domains (for ?8)
-- ?11     the cursor's created_at, as stored; NULL: the first page
-- ?12     the cursor's meeting ID
-- ?13     the most rows; -1 for every match
WITH RECURSIVE folder_tree (id) AS (
    SELECT value FROM json_each(?6)
    UNION
    SELECT child.id FROM folders AS child JOIN folder_tree AS tree ON child.parent_id = tree.id
),
-- Meetings someone from outside attended: an address that is neither the
-- user's nor at one of their work domains (profile::InternalDomains).
outsiders (meeting_id) AS (
    SELECT a.meeting_id
    FROM meeting_attendees AS a
    JOIN people AS p ON p.id = a.person_id
    WHERE p.email IS NOT NULL
        AND p.is_me = 0
        AND p.email NOT IN (SELECT value FROM json_each(?9))
        AND NOT EXISTS (
            SELECT 1 FROM json_each(?10) AS domain
            WHERE p.email LIKE '%@' || domain.value OR p.email LIKE '%.' || domain.value
        )
)
SELECT
    m.id AS "id!: MeetingId",
    m.title AS "title!",
    m.created_at AS "cursor_at!: String",
    m.created_at AS "created_at!: DateTime<Utc>",
    m.started_at AS "started_at: DateTime<Utc>",
    m.duration_s AS "duration: Seconds",
    m.status AS "status!: MeetingStatus"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND (?11 IS NULL OR m.created_at < ?11 OR (m.created_at = ?11 AND m.id < ?12))
    AND (?1 IS NULL OR julianday(m.created_at) >= julianday(?1))
    AND (?2 IS NULL OR julianday(m.created_at) < julianday(?2))
    AND (
        json_array_length(?3) = 0
        OR EXISTS (
            SELECT 1 FROM meeting_attendees AS a
            WHERE a.meeting_id = m.id AND a.person_id IN (SELECT value FROM json_each(?3))
        )
        OR EXISTS (
            SELECT 1 FROM speakers AS s
            WHERE s.meeting_id = m.id AND s.person_id IN (SELECT value FROM json_each(?3))
        )
    )
    AND (
        json_array_length(?4) = 0
        OR EXISTS (
            SELECT 1 FROM meeting_attendees AS a
            JOIN people AS p ON p.id = a.person_id
            WHERE a.meeting_id = m.id AND p.company_id IN (SELECT value FROM json_each(?4))
        )
    )
    AND (
        json_array_length(?5) = 0
        OR EXISTS (
            SELECT 1 FROM meeting_tags AS t
            WHERE t.meeting_id = m.id AND t.tag_id IN (SELECT value FROM json_each(?5))
        )
    )
    AND (
        json_array_length(?6) = 0
        OR EXISTS (
            SELECT 1 FROM meeting_folders AS f
            WHERE f.meeting_id = m.id AND f.folder_id IN (SELECT id FROM folder_tree)
        )
    )
    AND (?7 = 0 OR EXISTS (SELECT 1 FROM action_items AS i WHERE i.meeting_id = m.id))
    AND (
        ?8 IS NULL
        OR (?8 = 'external' AND m.id IN (SELECT meeting_id FROM outsiders))
        OR (
            ?8 = 'internal'
            AND m.id NOT IN (SELECT meeting_id FROM outsiders)
            AND EXISTS (
                SELECT 1 FROM meeting_attendees AS a
                JOIN people AS p ON p.id = a.person_id
                WHERE a.meeting_id = m.id AND p.email IS NOT NULL
            )
        )
    )
ORDER BY m.created_at DESC, m.id DESC
LIMIT ?13
