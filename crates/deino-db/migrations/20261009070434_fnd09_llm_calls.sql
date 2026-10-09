-- FND-09: one row per LLM provider call, for the usage page (PARITY_PLAN §4.3).
--
-- Usage metadata only: route, provider, model, token counts, latency and
-- outcome. Never prompt or response content, not even in debug builds.
-- Written by `deino_llm::SqliteLlmCallRepo` (one INSERT per call, through the
-- writer), read by the usage page. No foreign keys: a call is not tied to a
-- meeting, and the log outlives anything it served.

CREATE TABLE llm_calls (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- When the call started (RFC 3339 UTC).
    started_at TEXT NOT NULL,
    -- `deino_llm::Route`.
    route TEXT NOT NULL CHECK (route IN (
        'notes.enhance', 'notes.edit', 'chat', 'extract.action_items',
        'extract.insights', 'tagging', 'title', 'entity.summary', 'brief',
        'followup.email', 'embed'
    )),
    -- `deino_llm::ProviderId`.
    provider TEXT NOT NULL CHECK (provider IN (
        'anthropic', 'openai', 'groq', 'openrouter', 'custom_openai', 'ollama', 'builtin'
    )),
    -- The provider's model ID.
    model TEXT NOT NULL CHECK (length(model) > 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    -- From the request to the last event, in milliseconds.
    latency_ms INTEGER NOT NULL CHECK (latency_ms >= 0),
    -- `deino_llm::CallOutcome`.
    outcome TEXT NOT NULL CHECK (outcome IN ('ok', 'error', 'cancelled')),
    -- `deino_llm::LlmErrorKind`, set exactly when the call failed.
    error_kind TEXT CHECK (error_kind IN (
        'no_key', 'auth', 'rate_limited', 'overloaded', 'context_too_long',
        'rejected', 'provider', 'network', 'malformed', 'truncated',
        'unavailable', 'unsupported', 'invalid_output', 'tool_loop_exceeded',
        'credentials', 'routes', 'cancelled'
    )),
    CHECK ((outcome = 'error') = (error_kind IS NOT NULL))
) STRICT;

-- The usage page lists and sums calls newest first, optionally per route.
CREATE INDEX idx_llm_calls_started_at ON llm_calls (started_at);
CREATE INDEX idx_llm_calls_route_started_at ON llm_calls (route, started_at);
