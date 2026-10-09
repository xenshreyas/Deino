-- One meeting's notes; `meeting_id` is the primary key.
SELECT
    meeting_id AS "meeting_id!: MeetingId",
    notes_json,
    notes_markdown,
    created_at AS "created_at!: DateTime<Utc>",
    updated_at AS "updated_at!: DateTime<Utc>"
FROM meeting_notes
WHERE meeting_id = ?1
