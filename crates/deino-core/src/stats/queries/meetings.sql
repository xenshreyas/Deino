-- The counted meetings (the stats_meetings view) that started at or after
-- ?1, or every one when ?1 is NULL. julianday() compares instants, so
-- legacy rows saved in other timestamp spellings still filter correctly;
-- the scan is the benchmarked cost of computing on read (stats_bench.rs).
SELECT
    m.id AS "id!: MeetingId",
    m.at AS "at!: DateTime<Utc>",
    m.duration_s AS "duration_s: i64"
FROM stats_meetings AS m
WHERE ?1 IS NULL OR julianday(m.at) >= julianday(?1)
