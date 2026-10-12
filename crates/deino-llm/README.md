# deino-llm

## Role

Deino's LLM gateway (PARITY_PLAN §4.3, ADR 0017): the one way any code calls
an LLM. Every call names a task route; the gateway resolves it to a provider
and model, streams the answer, and meters the call into `llm_calls`.

## Owns

- `Gateway` (`gateway.rs`): `complete`, `stream`, `structured::<T>()`
  (`structured.rs`: schemars draft-07 with inlined subschemas, one repair
  retry; `structured_as` takes a schema built at run time, for AI-06's
  per-insight fields), `with_tools` (`tools.rs`: typed `Tool`s, a step-limited loop),
  `embed` (always local), and usage metering (`gateway/metering.rs`).
- `Route` and `RouteTier` (`route.rs`), `Router` (`router.rs`): per-route
  user choices over the §4.3 defaults (Claude Sonnet/Haiku with a key, the
  built-in model without, `embed` always local).
- `LlmProvider` (`provider.rs`, one required method: `stream`) and the
  adapters in `providers/`: Anthropic (Messages API; JSON answers are
  structured outputs, `output_config.format`, under a schema rewritten to
  the subset the API accepts by `providers/anthropic/schema.rs`; never a
  forced `tool_choice`, never `temperature`),
  OpenAI-compatible (`Flavor`: OpenAI, Groq, OpenRouter, custom; Ollama's
  `/v1`), Ollama (models, context windows, pulls), BuiltIn (llama-helper,
  `providers/builtin/protocol.rs` is the app's half of its protocol).
- `CompletionEvent` (`Text | ToolUse | Usage | Stop`), `Completion`,
  `LlmError` with `retry_after` and `is_retryable`.
- The SSE parser (`sse.rs`) every HTTP provider shares, with a property test.
- `SystemContext` (`context.rs`): standing context the gateway puts in front
  of every system prompt (`Gateway::with_system_context`; the app wires the
  user's profile, NOT-05).
- Ports: `HttpTransport`, `CredentialSource`, `EndpointSource`,
  `RouteStore`, `LocalModels`, `UsageRecorder`, `ModelCatalog`, `LlmCallRepo`
  (`SqliteLlmCallRepo` writes `llm_calls`).
- `LlmAdmin` (`admin.rs`): the Settings → AI service.
- Built-in prompts (`prompts.rs` + `prompts/*.md.jinja`, askama, insta
  snapshots) and user templates (`user_templates.rs`, minijinja, strict, with
  an allowlist), both after anarlog's `template-app` (MIT).
- Test support (feature `test-support`): `MockProvider` keyed by route
  (scripted text, tool calls, errors), `mock_gateway`, `FakeHttp`,
  `FakeLocalModels`, `MemoryRouteStore`, `MemoryUsage`, `StaticCredentials`,
  and `providers::anthropic::schema::constrained_schema` (proves a real
  schema reaches Anthropic as a valid structured output).
- Extraction answers (`extraction.rs`): `ActionItemsAnswer` (AI-05), the
  shape `extract.action_items` returns, and the prompt-line helpers
  (`meeting_header`, `attendee_line`, `transcript_line`) callers use to
  number the transcript the prompt cites.
- The eval harness: `tests/llm_eval/` (libtest-mimic, `pnpm eval <route>`;
  `extract.action_items` scores precision and recall against the fixtures'
  `groundTruth`; `notes.enhance` runs `deino_core::enhance` end to end over
  every seeded fixture meeting, a dev-only dependency on `deino-core`; live
  with `DEINO_EVAL=1 ANTHROPIC_API_KEY=…`).

## Does Not Own

- HTTP clients: the app implements `HttpTransport` over a `deino-net`
  `ScopedClient` per provider service (FND-12).
- Keys, endpoints and route choices: `deino_core::settings::LlmAccess` over
  the Keychain (`deino-secrets`) and the settings file.
- The llama-helper process, model files and chat templates: the app's
  `llm::local` adapter over `summary/summary_engine`.
- IPC: the app's `llm` plugin (`features/llm.rs`).
- Feature logic that uses LLMs (`deino_core::enhance` for notes and
  titles, later chat and extraction). The `notes.enhance` prompts live here
  (`prompts/notes_enhance*.jinja`); the engine that fills and checks them is
  `deino_core::enhance`.

## Invariants

- No dependency on `tauri` or `reqwest` (outside the eval binary's live runner).
- Nothing logs prompt or answer content, or a key: only routes, providers,
  models, counts and error kinds. `ApiKey`'s `Debug` is redacted, error
  messages quote providers' short error texts, never bodies.
- Every provider call becomes exactly one `llm_calls` row (finished, failed,
  truncated or cancelled), with no content.
- A provider stream ends with exactly one `Stop`, or with an `Err`.
- Structured answer types must not be recursive: Anthropic's structured
  outputs refuse recursive schemas, and such a type would fall back to an
  unconstrained, prompt-only JSON answer.
- New routes need a migration widening the `llm_calls.route` CHECK.
