-- An insight's runs on meetings outside the trash, newest meeting first,
-- with each meeting's title and start (the primary key narrows to the insight).
SELECT
    r.insight_id AS "insight_id: InsightId",
    r.meeting_id AS "meeting_id: MeetingId",
    r.status AS "status: InsightStatus",
    r.instances_json AS "answers: Json<Vec<InsightAnswer>>",
    r.revision,
    r.error,
    r.requested_at AS "requested_at: DateTime<Utc>",
    r.generated_at AS "generated_at: DateTime<Utc>",
    m.title AS meeting_title,
    coalesce(m.started_at, m.created_at) AS "meeting_started_at!: DateTime<Utc>"
FROM insight_results AS r
JOIN meetings AS m ON m.id = r.meeting_id
WHERE r.insight_id = ?1 AND m.deleted_at IS NULL
ORDER BY coalesce(m.started_at, m.created_at) DESC, m.id
