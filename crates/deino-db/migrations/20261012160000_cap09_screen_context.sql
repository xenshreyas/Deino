-- CAP-09 phase B: the text of each snapshot (stage `screen_context`) and
-- the `vision.describe` LLM route.

-- `snapshots.ocr_text`: what Apple Vision read in the picture ('' when it
-- found none); NULL until `screen_context` has read it, so a re-run reads
-- only new snapshots. `snapshots.description`: the optional
-- `vision.describe` answer (Settings → Screen capture → "Describe
-- snapshots with Claude"), NULL when not described. NOT-02's notes read
-- both, time-aligned with the transcript. The stored text is never
-- redacted; NOT-09 redacts what reaches the notes.
ALTER TABLE snapshots ADD COLUMN ocr_text TEXT;
ALTER TABLE snapshots ADD COLUMN description TEXT;

-- llm_calls: widen the route CHECK with `vision.describe`
-- (deino_llm::Route::VisionDescribe). SQLite cannot alter a CHECK, so the
-- table is rebuilt; nothing references it.
CREATE TABLE llm_calls_new (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    started_at TEXT NOT NULL,
    route TEXT NOT NULL CHECK (route IN (
        'notes.enhance', 'notes.edit', 'chat', 'extract.action_items',
        'extract.insights', 'tagging', 'title', 'entity.summary', 'brief',
        'followup.email', 'recap', 'vision.describe', 'embed'
    )),
    provider TEXT NOT NULL CHECK (provider IN (
        'anthropic', 'openai', 'groq', 'openrouter', 'custom_openai', 'ollama', 'builtin'
    )),
    model TEXT NOT NULL CHECK (length(model) > 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    latency_ms INTEGER NOT NULL CHECK (latency_ms >= 0),
    outcome TEXT NOT NULL CHECK (outcome IN ('ok', 'error', 'cancelled')),
    error_kind TEXT CHECK (error_kind IN (
        'no_key', 'auth', 'rate_limited', 'overloaded', 'context_too_long',
        'rejected', 'provider', 'network', 'malformed', 'truncated',
        'unavailable', 'unsupported', 'invalid_output', 'tool_loop_exceeded',
        'credentials', 'routes', 'cancelled'
    )),
    CHECK ((outcome = 'error') = (error_kind IS NOT NULL))
) STRICT;

INSERT INTO llm_calls_new (
    id, started_at, route, provider, model, input_tokens, output_tokens, latency_ms, outcome,
    error_kind
)
SELECT
    id, started_at, route, provider, model, input_tokens, output_tokens, latency_ms, outcome,
    error_kind
FROM llm_calls;

DROP TABLE llm_calls;
ALTER TABLE llm_calls_new RENAME TO llm_calls;

CREATE INDEX idx_llm_calls_started_at ON llm_calls (started_at);
CREATE INDEX idx_llm_calls_route_started_at ON llm_calls (route, started_at);
