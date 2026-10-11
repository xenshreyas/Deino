-- ORG-03: related meetings match an exact title (Granola §6.5), so the
-- lookup finds the titles of meetings outside the trash by index
-- (`calendar::linking::SqliteRelatedRepo::candidates`). The series side
-- already has idx_meetings_series_id (FND-07).
CREATE INDEX idx_meetings_title ON meetings (title) WHERE deleted_at IS NULL;
