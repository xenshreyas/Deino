-- The tag chips of a page's meetings (?1, a JSON array of meeting IDs), by
-- name. One lookup per meeting through the meeting_tags primary key.
SELECT
    mt.meeting_id AS "meeting_id!: MeetingId",
    t.id AS "id!: TagId",
    t.name AS "name!",
    t.color AS "color!: PaletteColor"
FROM json_each(?1) AS page
JOIN meeting_tags AS mt ON mt.meeting_id = page.value
JOIN tags AS t ON t.id = mt.tag_id
ORDER BY mt.meeting_id, t.name COLLATE NOCASE, t.id
