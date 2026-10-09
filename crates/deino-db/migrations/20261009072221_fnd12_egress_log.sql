-- FND-12: the egress log, one row per outgoing HTTP request (and per redirect
-- hop), so Settings → Privacy → Network can show what left the Mac.
--
-- Privacy: a row holds where a request went and how big it was, never what
-- it carried. There is deliberately no column for a URL, a path, a query
-- string, a header or a body.
--
-- `id` is an INTEGER rowid rather than a UUIDv7: rows are append-only and
-- never referenced. It is the list order (newest first, through the primary
-- key, so no index is needed) and the pruning order: `SqliteEgressLog::append`
-- keeps the newest 10,000 rows in the same transaction as each insert.
--
-- `service` lists every `deino_domain::ServiceId` and `method` every
-- `deino_net::Method`; a new value needs a migration that rebuilds this table.
CREATE TABLE egress_log (
    id INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    service TEXT NOT NULL CHECK (service IN (
        'llm_anthropic', 'llm_openai', 'llm_groq', 'llm_openrouter', 'llm_custom',
        'llm_ollama', 'hf_models', 'google_calendar', 'google_drive', 'gmail',
        'microsoft_graph', 'slack', 'notion', 'linear', 'todoist', 'asana',
        'hubspot', 'attio', 'salesforce', 'web_search', 'webhooks'
    )),
    -- Host and, when it is not the scheme's default, port: `localhost:11434`.
    host TEXT NOT NULL CHECK (host <> '' AND instr(host, '/') = 0 AND instr(host, '?') = 0),
    method TEXT NOT NULL CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE')),
    -- The HTTP status; NULL when no response arrived (blocked or failed).
    status INTEGER CHECK (status BETWEEN 100 AND 599),
    outcome TEXT NOT NULL CHECK (outcome IN ('sent', 'failed', 'blocked')),
    bytes_up INTEGER NOT NULL CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL CHECK (bytes_down >= 0)
) STRICT;
