-- Meetings outside the trash that pass every filter, newest first.
--
-- Each parameter is a JSON array; an empty array does not filter. Within a
-- kind every listed value must hold (AND), except ?5, which lists meetings
-- to choose from.
--
-- ?1  person IDs: each attended (an attendee, or a speaker linked to them)
-- ?2  company IDs: someone from each attended
-- ?3  tag IDs: each is on the meeting
-- ?4  folder IDs: the meeting is in each folder or one of its subfolders
-- ?5  meeting IDs: the meeting is one of them
-- ?6  domains: an attendee's email is at the domain (or a subdomain), or
--     their company's domain is it
WITH RECURSIVE folder_tree (root, id) AS (
    SELECT value, value FROM json_each(?4)
    UNION
    SELECT tree.root, child.id
    FROM folders AS child
    JOIN folder_tree AS tree ON child.parent_id = tree.id
)
SELECT
    m.id AS "id!: MeetingId",
    m.title AS "title!",
    coalesce(m.started_at, m.created_at) AS "date!: DateTime<Utc>"
FROM meetings AS m
WHERE m.deleted_at IS NULL
    AND NOT EXISTS (
        SELECT 1 FROM json_each(?1) AS wanted
        WHERE NOT EXISTS (
            SELECT 1 FROM meeting_attendees AS a
            WHERE a.meeting_id = m.id AND a.person_id = wanted.value
        )
        AND NOT EXISTS (
            SELECT 1 FROM speakers AS s
            WHERE s.meeting_id = m.id AND s.person_id = wanted.value
        )
    )
    AND NOT EXISTS (
        SELECT 1 FROM json_each(?2) AS wanted
        WHERE NOT EXISTS (
            SELECT 1 FROM meeting_attendees AS a
            JOIN people AS p ON p.id = a.person_id
            WHERE a.meeting_id = m.id AND p.company_id = wanted.value
        )
    )
    AND NOT EXISTS (
        SELECT 1 FROM json_each(?3) AS wanted
        WHERE NOT EXISTS (
            SELECT 1 FROM meeting_tags AS t
            WHERE t.meeting_id = m.id AND t.tag_id = wanted.value
        )
    )
    AND NOT EXISTS (
        SELECT 1 FROM json_each(?4) AS wanted
        WHERE NOT EXISTS (
            SELECT 1 FROM meeting_folders AS f
            JOIN folder_tree AS tree ON tree.id = f.folder_id
            WHERE f.meeting_id = m.id AND tree.root = wanted.value
        )
    )
    AND (json_array_length(?5) = 0 OR m.id IN (SELECT value FROM json_each(?5)))
    AND NOT EXISTS (
        SELECT 1 FROM json_each(?6) AS wanted
        WHERE NOT EXISTS (
            SELECT 1 FROM meeting_attendees AS a
            JOIN people AS p ON p.id = a.person_id
            LEFT JOIN companies AS c ON c.id = p.company_id
            WHERE a.meeting_id = m.id
                AND (
                    p.email LIKE '%@' || wanted.value
                    OR p.email LIKE '%.' || wanted.value
                    OR c.domain = wanted.value
                )
        )
    )
ORDER BY coalesce(m.started_at, m.created_at) DESC, m.id DESC
