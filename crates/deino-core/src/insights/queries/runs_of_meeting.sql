-- A meeting's insight runs (`idx_insight_results_meeting`).
SELECT
    insight_id AS "insight_id: InsightId",
    meeting_id AS "meeting_id: MeetingId",
    status AS "status: InsightStatus",
    instances_json AS "answers: Json<Vec<InsightAnswer>>",
    revision,
    error,
    requested_at AS "requested_at: DateTime<Utc>",
    generated_at AS "generated_at: DateTime<Utc>"
FROM insight_results
WHERE meeting_id = ?1
