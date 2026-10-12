-- ORG-09: the `recap` LLM route and the cached year in review.
--
-- Stats themselves are computed on read (PARITY_PLAN §4.0): counts, hours,
-- people, the heatmap and LLM usage come from `meetings`,
-- `meeting_attendees`, `people` and `llm_calls` with no stored aggregate.

-- llm_calls: widen the route CHECK with `recap` (deino_llm::Route::Recap).
-- SQLite cannot alter a CHECK, so the table is rebuilt; nothing references it.
CREATE TABLE llm_calls_new (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    started_at TEXT NOT NULL,
    route TEXT NOT NULL CHECK (route IN (
        'notes.enhance', 'notes.edit', 'chat', 'extract.action_items',
        'extract.insights', 'tagging', 'title', 'entity.summary', 'brief',
        'followup.email', 'recap', 'embed'
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

-- The year in review, generated once per year on the `recap` route and kept:
-- a write-once snapshot (§3A), so reopening the page never calls the model
-- again. Written by `deino_core::stats::SqliteRecapRepo::save` only.
CREATE TABLE year_recaps (
    -- The calendar year the recap covers, in the user's time zone.
    year INTEGER PRIMARY KEY NOT NULL CHECK (year BETWEEN 2000 AND 9999),
    -- The recap as Markdown.
    markdown TEXT NOT NULL CHECK (length(trim(markdown)) > 0),
    -- When it was generated (RFC 3339 UTC).
    generated_at TEXT NOT NULL
) STRICT;

-- The meetings Stats counts, defined once for every stats query: outside
-- the trash (ORG-07), recorded or being recorded rather than only
-- scheduled, and not the bundled sample (`deino_core::sample`, FND-11).
-- `at` is when the meeting started, or when it was saved for meetings from
-- before FND-07 kept a start time.
CREATE VIEW stats_meetings AS
SELECT
    id,
    title,
    coalesce(started_at, created_at) AS at,
    duration_s
FROM meetings
WHERE deleted_at IS NULL AND status <> 'scheduled' AND id <> 'meeting-sample';
