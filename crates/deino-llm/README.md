# deino-llm

## Role

Deino's LLM gateway (PARITY_PLAN §4.3, ADR 0017): the one way any code calls
an LLM. Every call names a task route; the gateway resolves it to a provider
and model, streams the answer, and meters the call into `llm_calls`.

## Owns

- `Gateway` (`gateway.rs`): `complete`, `stream`, `structured::<T>()`
  (`structured.rs`: schemars draft-07 with inlined subschemas, one repair
  retry), `with_tools` (`tools.rs`: typed `Tool`s, a step-limited loop),
  `embed` (always local), and usage metering (`gateway/metering.rs`).
- `Route` and `RouteTier` (`route.rs`), `Router` (`router.rs`): per-route
  user choices over the §4.3 defaults (Claude Sonnet/Haiku with a key, the
  built-in model without, `embed` always local).
- `LlmProvider` (`provider.rs`, one required method: `stream`) and the
  adapters in `providers/`: Anthropic (Messages API, tool trick for JSON),
  OpenAI-compatible (`Flavor`: OpenAI, Groq, OpenRouter, custom; Ollama's
  `/v1`), Ollama (models, context windows, pulls), BuiltIn (llama-helper,
  `providers/builtin/protocol.rs` is the app's half of its protocol).
- `CompletionEvent` (`Text | ToolUse | Usage | Stop`), `Completion`,
  `LlmError` with `retry_after` and `is_retryable`.
- The SSE parser (`sse.rs`) every HTTP provider shares, with a property test.
- Ports: `HttpTransport`, `CredentialSource`, `EndpointSource`,
  `RouteStore`, `LocalModels`, `UsageRecorder`, `ModelCatalog`, `LlmCallRepo`
  (`SqliteLlmCallRepo` writes `llm_calls`).
- `LlmAdmin` (`admin.rs`): the Settings → AI service.
- Built-in prompts (`prompts.rs` + `prompts/*.md.jinja`, askama, insta
  snapshots) and user templates (`user_templates.rs`, minijinja, strict, with
  an allowlist), both after anarlog's `template-app` (MIT).
- Test support (feature `test-support`): `MockProvider` keyed by route
  (scripted text, tool calls, errors), `mock_gateway`, `FakeHttp`,
  `FakeLocalModels`, `MemoryRouteStore`, `MemoryUsage`, `StaticCredentials`.
- The eval harness: `tests/llm_eval/` (libtest-mimic, `pnpm eval <route>`).

## Does Not Own

- HTTP clients: the app's transport implements `HttpTransport` (reqwest until
  FND-12's `deino-net`).
- Keys, endpoints and route choices: `deino_core::settings::LlmAccess` over
  the Keychain (`deino-secrets`) and the settings file.
- The llama-helper process, model files and chat templates: the app's
  `llm::local` adapter over `summary/summary_engine`.
- IPC: the app's `llm` plugin (`features/llm.rs`).
- Feature logic that uses LLMs (`deino_core::summaries`, later notes, chat,
  extraction).

## Invariants

- No dependency on `tauri` or `reqwest` (outside the eval binary's live runner).
- Nothing logs prompt or answer content, or a key: only routes, providers,
  models, counts and error kinds. `ApiKey`'s `Debug` is redacted, error
  messages quote providers' short error texts, never bodies.
- Every provider call becomes exactly one `llm_calls` row (finished, failed,
  truncated or cancelled), with no content.
- A provider stream ends with exactly one `Stop`, or with an `Err`.
- New routes need a migration widening the `llm_calls.route` CHECK.
