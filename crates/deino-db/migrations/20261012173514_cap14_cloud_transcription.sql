-- CAP-14: transcription after the meeting with ElevenLabs Scribe, on the
-- user's own API key (docs/adr/2026-10-12-cloud-transcription-option.md).
--
-- 1. The egress log learns the `elevenlabs` service. Its CHECK lists every
--    `ServiceId` (FND-12), and SQLite cannot change a CHECK in place, so the
--    table is rebuilt with its rows, as CAP-12 did. Ids are kept, so the
--    newest-first order and the pruning order do not change.
-- 2. `original_transcripts`: the transcript made on this Mac, kept when the
--    cloud transcript replaces it, so the user can bring it back. Retention
--    (PRV-01) deletes it with the meeting's transcript.

-- Every request Deino sent or blocked: when, which service and host, the method, status, outcome and sizes; never a path, query or body (FND-12).
CREATE TABLE egress_log_cap14 (
    id INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    service TEXT NOT NULL CHECK (service IN (
        'llm_anthropic', 'llm_openai', 'llm_groq', 'llm_openrouter', 'llm_custom',
        'llm_ollama', 'hf_models', 'google_calendar', 'google_drive', 'gmail',
        'microsoft_graph', 'slack', 'notion', 'linear', 'todoist', 'asana',
        'hubspot', 'attio', 'salesforce', 'jira', 'monday', 'clickup', 'trello',
        'affinity', 'zoho_crm', 'web_search', 'webhooks', 'favicons',
        'granola', 'circleback', 'fireflies', 'fathom', 'elevenlabs'
    )),
    host TEXT NOT NULL CHECK (host <> '' AND instr(host, '/') = 0 AND instr(host, '?') = 0),
    method TEXT NOT NULL CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE')),
    status INTEGER CHECK (status BETWEEN 100 AND 599),
    outcome TEXT NOT NULL CHECK (outcome IN ('sent', 'failed', 'blocked')),
    bytes_up INTEGER NOT NULL CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL CHECK (bytes_down >= 0)
) STRICT;

INSERT INTO egress_log_cap14 (id, at, service, host, method, status, outcome, bytes_up, bytes_down)
SELECT id, at, service, host, method, status, outcome, bytes_up, bytes_down FROM egress_log ORDER BY id;

DROP TABLE egress_log;
ALTER TABLE egress_log_cap14 RENAME TO egress_log;

-- The transcript made on this Mac that a cloud transcription replaced (CAP-14): one per meeting, the first one replaced, until the user restores it or retention deletes the transcript.
CREATE TABLE original_transcripts (
    meeting_id TEXT PRIMARY KEY NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    -- The replaced lines, in time order: a JSON array of
    -- `{channel, start, end, text, words}` (no speakers: they are carried
    -- over from the transcript in place when the lines are restored).
    lines TEXT NOT NULL CHECK (json_valid(lines) AND json_type(lines) = 'array'),
    -- The service whose transcript replaced it.
    replaced_by TEXT NOT NULL CHECK (replaced_by IN ('elevenlabs')),
    replaced_at TEXT NOT NULL
) STRICT, WITHOUT ROWID;

-- Retention deletes a meeting's transcript by stamping
-- `transcript_deleted_at` (PRV-01); the kept original goes with it.
CREATE TRIGGER original_transcripts_follow_retention
AFTER UPDATE OF transcript_deleted_at ON meetings
WHEN NEW.transcript_deleted_at IS NOT NULL
BEGIN
    DELETE FROM original_transcripts WHERE meeting_id = NEW.id;
END;
