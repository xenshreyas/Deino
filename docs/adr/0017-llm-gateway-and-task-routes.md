# 0017. Route every LLM call through one gateway, by task route

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-09 (`docs/references/architecture.md` §6 and (b) FND-09)

## Context

Before FND-09, summaries called providers through `summary/llm_client.rs`: a
13-argument function with one `match` per provider, a non-streaming request
each, an Ollama-only compatibility retry, and the built-in model reached
through a separate one-shot sidecar request. Model lists came from five more
legacy modules (`ollama/`, `openai/`, `anthropic/`, `groq/`, `openrouter/`),
each with its own HTTP client, cache and fallback list. Nothing recorded
usage, nothing could stream to the UI, and later cards (enhanced notes, chat,
extraction, tagging, briefs, embeddings) each need streaming, typed output
or tool calls. PARITY_PLAN §3.6 requires every call to go through one gateway
and name a task route; §4.3 fixes the routes and their defaults.

## Decision

- **A crate, `deino-llm`, Tauri-free and reqwest-free**, in three layers (the
  zed pattern, reimplemented: zed is reference-only):
  1. per-provider wire types (`providers/<provider>/wire.rs`);
  2. adapters implementing `LlmProvider`, whose only required method is
     `stream()` returning `CompletionEvent = Text | ToolUse | Usage | Stop`
     (`complete()` and `context_window()` are provided defaults);
  3. the provider-agnostic `Gateway`: `stream`, `complete`,
     `structured::<T: JsonSchema>()`, `with_tools(Toolbox)` and `embed`.
- **Task routes are an enum** (`notes.enhance`, `notes.edit`, `chat`,
  `extract.action_items`, `extract.insights`, `tagging`, `title`,
  `entity.summary`, `brief`, `followup.email`, `embed`), each with a tier
  (smart, fast, embedding). The `Router` resolves a route to a provider and
  model: the user's choice (`Settings::ai.routes`) or the §4.3 default
  (Claude Sonnet/Haiku with an Anthropic key, otherwise the first downloaded
  built-in model; `embed` always local and not pinnable). A prompt may pin a
  target, which the legacy summaries use for the model picked in their UI.
- **Structured output** uses each provider's strongest tool: Anthropic's
  forced tool (the "tool trick"), OpenAI-compatible `response_format:
  json_schema` with a 400/422 fallback to a prompt-only schema (Handy), and a
  llama.cpp grammar from the schema for the built-in model. Schemas are
  schemars draft-07 with subschemas inlined. One repair retry shows the model
  its answer and the parse error.
- **Errors** are a taxonomy (`LlmError`) with `retry_after` and
  `is_retryable`, user-safe messages and an `ErrorClass` mapping.
- **Ports for everything external**: `HttpTransport` (reqwest in the app until
  FND-12's `deino-net`), `CredentialSource`/`EndpointSource`/`RouteStore`
  (`deino_core::settings::LlmAccess` over the Keychain and the settings
  file), `LocalModels` (the sidecar), `UsageRecorder` (a channel to a writer
  task that inserts `llm_calls`).
- **Usage, never content**: every provider call becomes one `llm_calls` row
  (route, provider, model, tokens, latency, outcome, error kind), recorded on
  stop, failure, truncation or drop. Nothing logs prompts, answers or keys.
- **Prompts**: built-ins are askama `.md.jinja` templates with insta
  snapshots; user templates are minijinja with strict undefined variables and
  an allowlist (both after anarlog's `template-app`, MIT).
- **Streaming to the UI** uses a typed `tauri::ipc::Channel<LlmStreamEvent>`
  per request (the `llm` plugin's `test_route`), not a global event.
- **Tests** script answers by route with `MockProvider`; providers have wire
  tests over recorded SSE; evals (`pnpm eval <route>`) run against real keys
  outside CI.

## Alternatives

- **LLM calls in the frontend** (anarlog's Vercel AI SDK): rejected, because
  all egress must go through Rust (FND-12), and keys never reach the UI.
- **A crate per provider or a third-party multi-provider crate**: more
  surface than Deino needs, and none offers the tool trick, grammar output
  and route metering together.
- **Routes as strings**: typos would create silent routes; the enum gives the
  UI a typed table and the database a CHECK constraint.
- **Recording usage asynchronously in the stream's `Drop`**: impossible
  without spawning in `Drop`; a synchronous recorder feeding a writer task
  records cancelled calls too.

## Consequences

- A feature adds an LLM task by adding a route (and a migration widening the
  `llm_calls.route` CHECK), then calls the gateway; it never touches a
  provider. Adding a provider is one adapter module and one registration in
  `frontend/src-tauri/src/llm.rs`.
- Every feature is testable offline with `MockProvider` and `mock_gateway`.
- FND-12 swaps the reqwest transport for `deino-net` without changing the
  crate; the egress log and offline mode then cover LLM traffic.
- The legacy summaries now run through the gateway (`deino_core::summaries`);
  FND-08 makes them a pipeline stage and NOT-02 replaces them.
