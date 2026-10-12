-- CAP-12: meetings imported from other note-takers (Granola, Circleback,
-- Fireflies, Fathom) and from folders of JSON or Markdown files.
--
-- 1. The egress log learns the four import services. Its CHECK lists every
--    `ServiceId` (FND-12), and SQLite cannot change a CHECK in place, so the
--    table is rebuilt with its rows. Ids are kept, so the newest-first order
--    and the pruning order do not change.
-- 2. `imported_meetings`, the de-duplication ledger.

-- Every request Deino sent or blocked: when, which service and host, the method, status, outcome and sizes; never a path, query or body (FND-12).
CREATE TABLE egress_log_cap12 (
    id INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    service TEXT NOT NULL CHECK (service IN (
        'llm_anthropic', 'llm_openai', 'llm_groq', 'llm_openrouter', 'llm_custom',
        'llm_ollama', 'hf_models', 'google_calendar', 'google_drive', 'gmail',
        'microsoft_graph', 'slack', 'notion', 'linear', 'todoist', 'asana',
        'hubspot', 'attio', 'salesforce', 'jira', 'monday', 'clickup', 'trello',
        'affinity', 'zoho_crm', 'web_search', 'webhooks', 'favicons',
        'granola', 'circleback', 'fireflies', 'fathom'
    )),
    host TEXT NOT NULL CHECK (host <> '' AND instr(host, '/') = 0 AND instr(host, '?') = 0),
    method TEXT NOT NULL CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE')),
    status INTEGER CHECK (status BETWEEN 100 AND 599),
    outcome TEXT NOT NULL CHECK (outcome IN ('sent', 'failed', 'blocked')),
    bytes_up INTEGER NOT NULL CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL CHECK (bytes_down >= 0)
) STRICT;

INSERT INTO egress_log_cap12 (id, at, service, host, method, status, outcome, bytes_up, bytes_down)
SELECT id, at, service, host, method, status, outcome, bytes_up, bytes_down FROM egress_log ORDER BY id;

DROP TABLE egress_log;
ALTER TABLE egress_log_cap12 RENAME TO egress_log;

-- Meetings imported from another note-taker or a folder, by their ID there: the de-duplication ledger, so importing twice adds nothing.
CREATE TABLE imported_meetings (
    -- `granola`, `circleback`, `fireflies`, `fathom` or `folder`.
    source TEXT NOT NULL CHECK (source IN ('granola', 'circleback', 'fireflies', 'fathom', 'folder')),
    -- The meeting's ID in the source; for a folder, the SHA-256 of the file.
    source_id TEXT NOT NULL CHECK (length(source_id) BETWEEN 1 AND 200),
    -- NULL once the meeting is deleted: the row stays, so a later import
    -- never brings back a meeting the user removed (like CAP-11's ledger).
    meeting_id TEXT REFERENCES meetings (id) ON DELETE SET NULL,
    imported_at TEXT NOT NULL,
    PRIMARY KEY (source, source_id)
) STRICT, WITHOUT ROWID;

-- The foreign key.
CREATE INDEX idx_imported_meetings_meeting_id ON imported_meetings (meeting_id);
