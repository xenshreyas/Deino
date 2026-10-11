-- ORG-08: the keyword search index (PARITY_PLAN §4.4, ADR 0029).
--
-- Two parts, both denormalised copies kept in sync by triggers in this file
-- (§3A "Database design"):
--
-- - `search_documents`: one row per searchable thing, derived from its source
--   of truth by the triggers below, in the same transaction as every write
--   to the source (legacy writes included). It is the FTS index's external
--   content table, so FTS5 gets a stable integer rowid per document and the
--   triggers find a document by (kind, ref_id) through an index instead of
--   scanning the full-text table.
-- - `search_fts`: the FTS5 index over `search_documents` (porter stemming on
--   unicode61, diacritics folded, 2- and 3-character prefix indexes for
--   type-ahead). The three triggers on `search_documents` keep it in step.
--
-- | kind          | source of truth                         | ref_id      | title         | body                    | ts_ms     |
-- |---------------|-----------------------------------------|-------------|---------------|-------------------------|-----------|
-- | `meeting`     | meetings.title                          | meeting id  | title         | ''                      | NULL      |
-- | `note`        | meeting_notes.notes_markdown (Readable) | meeting id  | ''            | the Markdown            | NULL      |
-- | `summary`     | summary_processes.result ($.markdown)   | meeting id  | ''            | the Markdown            | NULL      |
-- | `segment`     | transcript_segments (not soft-deleted)  | segment id  | ''            | text                    | start_ms  |
-- | `action_item` | action_items                            | item id     | title         | description             | NULL      |
-- | `person`      | people (not merged)                     | person id   | name          | email and job title     | NULL      |
-- | `company`     | companies                               | company id  | name          | domain                  | NULL      |
-- | `folder`      | folders                                 | folder id   | name          | description             | NULL      |
--
-- Trashed meetings stay indexed (restoring one needs no reindex); queries
-- join `meetings` and skip `deleted_at IS NOT NULL`. Hard deletes cascade
-- through `meeting_id`. `deino_core::search::SqliteSearchIndex::reindex_all`
-- re-derives every document from these same rules and rebuilds the index.
--
-- Each source trigger "refreshes" a document: it deletes the old one and
-- inserts the new one only when there is something to find, so an emptied
-- note or a soft-deleted segment simply leaves the index.

CREATE TABLE search_documents (
    -- The FTS5 rowid. Never referenced from another table.
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (
        kind IN ('meeting', 'note', 'summary', 'segment', 'action_item', 'person', 'company', 'folder')
    ),
    ref_id TEXT NOT NULL CHECK (length(ref_id) > 0),
    -- The meeting the document belongs to; NULL for people, companies,
    -- folders and standalone action items.
    meeting_id TEXT REFERENCES meetings (id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    -- Where in the recording a transcript segment starts.
    ts_ms INTEGER CHECK (ts_ms IS NULL OR ts_ms >= 0),
    UNIQUE (kind, ref_id)
) STRICT;

-- The foreign key, and a meeting's documents for `refresh_meeting`.
CREATE INDEX idx_search_documents_meeting_id ON search_documents (meeting_id, kind);

CREATE VIRTUAL TABLE search_fts USING fts5 (
    kind UNINDEXED,
    ref_id UNINDEXED,
    meeting_id UNINDEXED,
    title,
    body,
    ts_ms UNINDEXED,
    content = 'search_documents',
    content_rowid = 'id',
    tokenize = 'porter unicode61 remove_diacritics 2',
    prefix = '2 3'
);

-- Ranking: BM25 over the indexed columns only, a title match counting four
-- times a body match (weights in column order: kind, ref_id, meeting_id,
-- title, body, ts_ms). As the table's default `rank`, queries can use FTS5's
-- fast `ORDER BY rank LIMIT` path.
INSERT INTO search_fts (search_fts, rank) VALUES ('rank', 'bm25(0.0, 0.0, 0.0, 4.0, 1.0, 0.0)');

-- External content: the index mirrors `search_documents` row for row.

CREATE TRIGGER search_documents_after_insert AFTER INSERT ON search_documents
BEGIN
    INSERT INTO search_fts (rowid, kind, ref_id, meeting_id, title, body, ts_ms)
    VALUES (NEW.id, NEW.kind, NEW.ref_id, NEW.meeting_id, NEW.title, NEW.body, NEW.ts_ms);
END;

CREATE TRIGGER search_documents_after_delete AFTER DELETE ON search_documents
BEGIN
    INSERT INTO search_fts (search_fts, rowid, kind, ref_id, meeting_id, title, body, ts_ms)
    VALUES ('delete', OLD.id, OLD.kind, OLD.ref_id, OLD.meeting_id, OLD.title, OLD.body, OLD.ts_ms);
END;

CREATE TRIGGER search_documents_after_update AFTER UPDATE ON search_documents
BEGIN
    INSERT INTO search_fts (search_fts, rowid, kind, ref_id, meeting_id, title, body, ts_ms)
    VALUES ('delete', OLD.id, OLD.kind, OLD.ref_id, OLD.meeting_id, OLD.title, OLD.body, OLD.ts_ms);
    INSERT INTO search_fts (rowid, kind, ref_id, meeting_id, title, body, ts_ms)
    VALUES (NEW.id, NEW.kind, NEW.ref_id, NEW.meeting_id, NEW.title, NEW.body, NEW.ts_ms);
END;

-- Meetings -------------------------------------------------------------------

CREATE TRIGGER search_meetings_after_insert AFTER INSERT ON meetings
BEGIN
    INSERT INTO search_documents (kind, ref_id, meeting_id, title)
    VALUES ('meeting', NEW.id, NEW.id, NEW.title);
END;

CREATE TRIGGER search_meetings_after_rename AFTER UPDATE OF title ON meetings
WHEN NEW.title IS NOT OLD.title
BEGIN
    UPDATE search_documents SET title = NEW.title WHERE kind = 'meeting' AND ref_id = NEW.id;
END;

-- A meeting's own documents leave with it through the foreign key.

-- User notes -----------------------------------------------------------------

CREATE TRIGGER search_meeting_notes_after_insert AFTER INSERT ON meeting_notes
WHEN trim(coalesce(NEW.notes_markdown, '')) <> ''
BEGIN
    INSERT INTO search_documents (kind, ref_id, meeting_id, body)
    VALUES ('note', NEW.meeting_id, NEW.meeting_id, NEW.notes_markdown);
END;

CREATE TRIGGER search_meeting_notes_after_update AFTER UPDATE OF notes_markdown, meeting_id ON meeting_notes
BEGIN
    DELETE FROM search_documents WHERE kind = 'note' AND ref_id = OLD.meeting_id;
    INSERT INTO search_documents (kind, ref_id, meeting_id, body)
    SELECT 'note', NEW.meeting_id, NEW.meeting_id, NEW.notes_markdown
    WHERE trim(coalesce(NEW.notes_markdown, '')) <> '';
END;

CREATE TRIGGER search_meeting_notes_after_delete AFTER DELETE ON meeting_notes
BEGIN
    DELETE FROM search_documents WHERE kind = 'note' AND ref_id = OLD.meeting_id;
END;

-- Summaries (the legacy `summary_processes` result until NOT-02) -------------
-- `result` is the legacy JSON (`{"markdown": ..., ...}`) or NULL while the
-- summary runs. Nested CASEs, because SQLite may evaluate the terms of a
-- WHERE in any order and `json_type` fails on text that is not JSON.

CREATE TRIGGER search_summaries_after_insert AFTER INSERT ON summary_processes
BEGIN
    INSERT INTO search_documents (kind, ref_id, meeting_id, body)
    SELECT 'summary', NEW.meeting_id, NEW.meeting_id, markdown
    FROM (SELECT CASE WHEN json_valid(NEW.result) THEN CASE WHEN json_type(NEW.result, '$.markdown') = 'text' THEN json_extract(NEW.result, '$.markdown') END END AS markdown)
    WHERE trim(coalesce(markdown, '')) <> '';
END;

CREATE TRIGGER search_summaries_after_update AFTER UPDATE OF result, meeting_id ON summary_processes
BEGIN
    DELETE FROM search_documents WHERE kind = 'summary' AND ref_id = OLD.meeting_id;
    INSERT INTO search_documents (kind, ref_id, meeting_id, body)
    SELECT 'summary', NEW.meeting_id, NEW.meeting_id, markdown
    FROM (SELECT CASE WHEN json_valid(NEW.result) THEN CASE WHEN json_type(NEW.result, '$.markdown') = 'text' THEN json_extract(NEW.result, '$.markdown') END END AS markdown)
    WHERE trim(coalesce(markdown, '')) <> '';
END;

CREATE TRIGGER search_summaries_after_delete AFTER DELETE ON summary_processes
BEGIN
    DELETE FROM search_documents WHERE kind = 'summary' AND ref_id = OLD.meeting_id;
END;

-- Transcript segments --------------------------------------------------------

CREATE TRIGGER search_segments_after_insert AFTER INSERT ON transcript_segments
WHEN NEW.is_deleted = 0 AND trim(NEW.text) <> ''
BEGIN
    INSERT INTO search_documents (kind, ref_id, meeting_id, body, ts_ms)
    VALUES ('segment', NEW.id, NEW.meeting_id, NEW.text, NEW.start_ms);
END;

-- Speaker changes (diarization) do not touch the document.
CREATE TRIGGER search_segments_after_update AFTER UPDATE OF text, start_ms, is_deleted, meeting_id ON transcript_segments
BEGIN
    DELETE FROM search_documents WHERE kind = 'segment' AND ref_id = OLD.id;
    INSERT INTO search_documents (kind, ref_id, meeting_id, body, ts_ms)
    SELECT 'segment', NEW.id, NEW.meeting_id, NEW.text, NEW.start_ms
    WHERE NEW.is_deleted = 0 AND trim(NEW.text) <> '';
END;

CREATE TRIGGER search_segments_after_delete AFTER DELETE ON transcript_segments
BEGIN
    DELETE FROM search_documents WHERE kind = 'segment' AND ref_id = OLD.id;
END;

-- Action items ---------------------------------------------------------------

CREATE TRIGGER search_action_items_after_insert AFTER INSERT ON action_items
BEGIN
    INSERT INTO search_documents (kind, ref_id, meeting_id, title, body)
    VALUES ('action_item', NEW.id, NEW.meeting_id, NEW.title, NEW.description);
END;

CREATE TRIGGER search_action_items_after_update AFTER UPDATE OF title, description, meeting_id ON action_items
BEGIN
    DELETE FROM search_documents WHERE kind = 'action_item' AND ref_id = OLD.id;
    INSERT INTO search_documents (kind, ref_id, meeting_id, title, body)
    VALUES ('action_item', NEW.id, NEW.meeting_id, NEW.title, NEW.description);
END;

CREATE TRIGGER search_action_items_after_delete AFTER DELETE ON action_items
BEGIN
    DELETE FROM search_documents WHERE kind = 'action_item' AND ref_id = OLD.id;
END;

-- People ---------------------------------------------------------------------

CREATE TRIGGER search_people_after_insert AFTER INSERT ON people
WHEN NEW.merged_into IS NULL
BEGIN
    INSERT INTO search_documents (kind, ref_id, title, body)
    VALUES ('person', NEW.id, NEW.name, trim(coalesce(NEW.email, '') || ' ' || coalesce(NEW.title, '')));
END;

CREATE TRIGGER search_people_after_update AFTER UPDATE OF name, email, title, merged_into ON people
BEGIN
    DELETE FROM search_documents WHERE kind = 'person' AND ref_id = OLD.id;
    INSERT INTO search_documents (kind, ref_id, title, body)
    SELECT 'person', NEW.id, NEW.name, trim(coalesce(NEW.email, '') || ' ' || coalesce(NEW.title, ''))
    WHERE NEW.merged_into IS NULL;
END;

CREATE TRIGGER search_people_after_delete AFTER DELETE ON people
BEGIN
    DELETE FROM search_documents WHERE kind = 'person' AND ref_id = OLD.id;
END;

-- Companies ------------------------------------------------------------------

CREATE TRIGGER search_companies_after_insert AFTER INSERT ON companies
BEGIN
    INSERT INTO search_documents (kind, ref_id, title, body)
    VALUES ('company', NEW.id, NEW.name, coalesce(NEW.domain, ''));
END;

CREATE TRIGGER search_companies_after_update AFTER UPDATE OF name, domain ON companies
BEGIN
    UPDATE search_documents SET title = NEW.name, body = coalesce(NEW.domain, '')
    WHERE kind = 'company' AND ref_id = NEW.id;
END;

CREATE TRIGGER search_companies_after_delete AFTER DELETE ON companies
BEGIN
    DELETE FROM search_documents WHERE kind = 'company' AND ref_id = OLD.id;
END;

-- Folders --------------------------------------------------------------------

CREATE TRIGGER search_folders_after_insert AFTER INSERT ON folders
BEGIN
    INSERT INTO search_documents (kind, ref_id, title, body)
    VALUES ('folder', NEW.id, NEW.name, NEW.description);
END;

CREATE TRIGGER search_folders_after_update AFTER UPDATE OF name, description ON folders
BEGIN
    UPDATE search_documents SET title = NEW.name, body = NEW.description
    WHERE kind = 'folder' AND ref_id = NEW.id;
END;

-- Subfolders deleted by the parent's cascade fire this trigger too.
CREATE TRIGGER search_folders_after_delete AFTER DELETE ON folders
BEGIN
    DELETE FROM search_documents WHERE kind = 'folder' AND ref_id = OLD.id;
END;

-- Existing data --------------------------------------------------------------
-- The same rules as the triggers, for every row written before this
-- migration (`SqliteSearchIndex::reindex_all` runs the same statements).

INSERT INTO search_documents (kind, ref_id, meeting_id, title)
SELECT 'meeting', id, id, title FROM meetings;

INSERT INTO search_documents (kind, ref_id, meeting_id, body)
SELECT 'note', meeting_id, meeting_id, notes_markdown FROM meeting_notes
WHERE trim(coalesce(notes_markdown, '')) <> '';

INSERT INTO search_documents (kind, ref_id, meeting_id, body)
SELECT 'summary', meeting_id, meeting_id, markdown
FROM (SELECT meeting_id, CASE WHEN json_valid(result) THEN CASE WHEN json_type(result, '$.markdown') = 'text' THEN json_extract(result, '$.markdown') END END AS markdown FROM summary_processes)
WHERE trim(coalesce(markdown, '')) <> '';

INSERT INTO search_documents (kind, ref_id, meeting_id, body, ts_ms)
SELECT 'segment', id, meeting_id, text, start_ms FROM transcript_segments
WHERE is_deleted = 0 AND trim(text) <> '';

INSERT INTO search_documents (kind, ref_id, meeting_id, title, body)
SELECT 'action_item', id, meeting_id, title, description FROM action_items;

INSERT INTO search_documents (kind, ref_id, title, body)
SELECT 'person', id, name, trim(coalesce(email, '') || ' ' || coalesce(title, '')) FROM people
WHERE merged_into IS NULL;

INSERT INTO search_documents (kind, ref_id, title, body)
SELECT 'company', id, name, coalesce(domain, '') FROM companies;

INSERT INTO search_documents (kind, ref_id, title, body)
SELECT 'folder', id, name, description FROM folders;
