-- Titles of the counted meetings that started in [?1, ?2), oldest first,
-- at most ?3: all the year in review reads besides the numbers.
SELECT
    m.title AS "title!",
    m.at AS "at!: DateTime<Utc>"
FROM stats_meetings AS m
WHERE julianday(m.at) >= julianday(?1) AND julianday(m.at) < julianday(?2)
ORDER BY julianday(m.at), m.id
LIMIT ?3
