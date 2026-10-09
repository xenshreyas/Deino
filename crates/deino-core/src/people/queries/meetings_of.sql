-- The meetings outside the trash a person (?1) attended, newest first.
-- Filtered through idx_meeting_attendees_person_id and joined on the
-- meetings primary key (asserted in attendee_repo_tests.rs); one person's
-- meetings are few enough to sort.
SELECT
    m.id AS "id!: MeetingId",
    m.title,
    m.created_at AS "created_at: DateTime<Utc>",
    m.updated_at AS "updated_at: DateTime<Utc>"
FROM meeting_attendees AS a
JOIN meetings AS m ON m.id = a.meeting_id
WHERE a.person_id = ?1 AND m.deleted_at IS NULL
ORDER BY m.created_at DESC, m.id DESC
