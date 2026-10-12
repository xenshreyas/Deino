# FIX-anthropic-5-5 · Structured outputs instead of the tool trick on Claude 5.5

## Progress log (PARITY_PLAN §7.1)
Production bug: writing notes failed with "Anthropic rejected the request
(400): tool_choice: type "tool" and "any" are not supported for this model."
Claude Fable 5.1, Mythos 5.1, Opus 5.5, Sonnet 5.5 and the Haiku 5.5
generation refuse a forced `tool_choice`, and Deino's JSON answers (every
`Gateway::structured::<T>()` call: notes, edits, action items, insights,
tags, template picks, redaction checks, briefs, follow-ups) forced one tool
whose input schema was the answer's schema. Only the Anthropic adapter
changed; OpenAI-compatible, Ollama and the built-in model are untouched.

- **JSON answers are structured outputs** (`providers/anthropic/wire.rs`):
  `"output_config": {"format": {"type": "json_schema", "schema": …}}`, no
  `tools`, no `tool_choice`. Agentic text routes keep their tools with
  `tool_choice` absent (auto), as before.
- **Schema rewrite** (`providers/anthropic/schema.rs`, `schema/refs.rs`,
  pure, unit-tested): `output_schema(&Value) -> OutputSchema` sets
  `"additionalProperties": false` on every object schema (nested
  properties, `items` and tuple `items`, `anyOf`/`allOf` branches, `$defs`
  and `definitions`), drops `minimum`/`maximum`/`exclusive*`/`multipleOf`,
  `minLength`/`maxLength`, `minItems`/`maxItems`/`uniqueItems`/`contains`,
  `min/maxProperties`, drops any `format` outside date-time, time, date,
  duration, email, hostname, uri, ipv4, ipv6, uuid, and sends `oneOf` as
  `anyOf`. Property names, `enum`/`const`/`default` values and descriptions
  are never touched (the walk only enters schema positions).
- **Unconstrained fallback.** A schema the API cannot take (recursive, a
  `$ref` that does not resolve to `#` or a root definition, an open map
  `additionalProperties: {…}`/`true`/`patternProperties`, a `true`
  subschema, `oneOf` beside `anyOf`) is `OutputSchema::Unconstrained`: no
  `output_config`, and the system prompt gains "Reply with only one JSON value
  that matches this JSON schema…" plus the original schema; the gateway's
  parse and repair are the check. Logged at info with the schema name and
  reason only. Recursion is a cycle in the `$ref` graph (root + root
  definitions). `schemars` keeps a recursive type as a `$ref` and
  `json_schema_for` drops definitions, so a recursive Deino type surfaces as
  an unresolved `$ref`, which falls back the same way (tested with a
  recursive `Outline` type).
- **No `temperature` to Anthropic, ever.** Haiku 5.5 refuses non-default
  sampling and Sonnet 5.5's migration says to remove it. `Prompt.temperature`
  still reaches the other providers.
- **Decoder** (`decode.rs`): text deltas are the answer in both modes;
  thinking and signature deltas are dropped (Sonnet/Haiku 5.5 think
  adaptively by default, so a JSON answer usually follows a thinking block);
  `input_json_delta` only feeds real tool calls. A JSON answer that stops on
  `max_tokens` emits its usage at `message_delta` and ends the stream with
  `LlmError::Truncated` (retryable) instead of reaching the parser and the
  repair turn; a text answer cut off still ends with `Stop(MaxTokens)`.
  `refusal` keeps mapping to `StopReason::Refusal`.
- **`max_tokens` floor for JSON answers: 16 000** (`JSON_ANSWER_MIN_TOKENS`).
  Thinking counts against `max_tokens`, and the route caps were sized for the
  JSON alone (template pick 200, tagging 300, redaction check
  24/candidate, follow-up decide/draft, action items/insights/notes edit
  4 096, notes 8 192). A larger route cap is kept. The request streams, so
  there is no timeout risk, and the cap is a ceiling, not a spend. No route
  needs less: a small cap only buys a truncation.
- **Effort is not sent.** Sonnet 5.5 defaults to `high`, Haiku 5.5 to
  `medium`. `output_config.effort` would trim latency on extraction routes,
  but Haiku 4.5 (which a user can still pick, and which takes structured
  outputs) and Sonnet 4.5 refuse it, and notes quality wants the default. No
  `thinking` field is sent either (`disabled` is a 400 on Sonnet/Opus 5.5).
- **Tests.** `anthropic_tests.rs`: the JSON request body (exact:
  `output_config.format`, closed schema with `maxLength` dropped, floor
  `max_tokens`, no `tools`/`tool_choice`/`temperature` even when the request
  carries tools and a temperature); a cap above the floor is kept; a
  recursive schema goes to the system prompt; the JSON answer streams from
  text deltas after a thinking block (`anthropic_json_answer.sse`, re-recorded
  in the structured-output shape); `anthropic_json_max_tokens.sse` gives
  `Text, Usage, Err(Truncated)` for JSON and `Stop(MaxTokens)` for text; text
  and tool bodies are unchanged except that `temperature` is gone.
  `schema_tests.rs`: nested objects, `$defs`/`definitions`/`$ref`, arrays
  and tuple items, `anyOf`/`oneOf`, stripped keywords and formats,
  keyword-named properties, recursion through definitions and the root,
  shared non-recursive definitions, unsupported shapes.
- **Every real structured schema is checked** with
  `schema::constrained_schema` (feature `test-support`: constrained, and every
  object closed, no refused keyword or format, no `oneOf` left):
  `deino-core/src/structured_schemas_tests.rs` (`EnhancedDoc`, `EditAnswer`,
  `TaggingAnswer`, `ActionItemsAnswer`, `BriefAnswer`, `FollowUpDecision`,
  `FollowUpDraftAnswer`), plus `templates` `Pick`, `redaction` `Verdicts`
  and `insights::schema::answer_schema` (fixed, auto, free text) in their own
  test modules. None is recursive and none falls back. What the rewrite
  changes in them: `additionalProperties: false` on every object (all of
  them); `oneOf` → `anyOf` for documented unit enums (`BlockKind`, `Origin`,
  `EditAction`, `NewKind`, `ExtractedPriority`, `FollowUpVerdict`);
  `format: uint`/`minimum: 0` dropped from `Verdict.number`; `maxItems`
  dropped from a fixed-count insight (`validate.rs` already keeps at most
  `Cardinality::limit()` answers). Titles, entity summaries, recaps and
  speaker identification are not structured calls.

## Follow-ups (PARITY_PLAN §7.2)
- AI owner: small text caps with adaptive thinking on. `Route::Title`
  (`TITLE_MAX_TOKENS` 40), `vision.describe` (400) and the Settings → AI route
  test (64) are plain text routes, left unchanged here; on Sonnet/Haiku 5.5 a
  thinking block can use the whole cap and the title comes back empty
  (`clean_title` → `None`, so no error, just no auto title). Consider an
  Anthropic floor for text routes too, or `output_config.effort: "low"` gated
  on models that accept it.
- AI owner: optional per-route `effort` (`low` for extraction/classification
  routes on Sonnet/Haiku 5.5) once there is a model-capability table, since
  Haiku 4.5 and Sonnet 4.5 refuse the parameter.
- AI owner: a `refusal` on a JSON answer still goes through the repair turn
  and ends as `InvalidOutput`; a typed refusal error would be clearer.
- AI owner: structured outputs are documented for Fable 5/5.1, Mythos 5/5.1,
  Opus 5.5/5/4.8, Sonnet 5.5/5, Haiku 5.5 and 4.5. A user who picks an older
  model without structured-output support would now get a 400 where the tool
  trick worked; if that matters, gate on the model and keep the tool trick
  for pre-5.1 models.
- Owner QA: write notes on Sonnet 5.5 and Haiku 5.5 with a real key
  (enhance, edit by asking, action items, insights, tags, follow-up).

## Architecture (ARCHITECTURE.md)
- LLM gateway / Anthropic adapter: "JSON answers are Anthropic structured
  outputs (`output_config.format`) under a schema rewritten to the API's
  subset (`providers/anthropic/schema.rs`); a schema it cannot take
  (recursive, open map) is asked for in the system prompt. Anthropic never
  receives `tool_choice` or `temperature`; JSON answers get `max_tokens` ≥
  16 000 because 5.5 models think by default."
- ADR 0017 (line 39, "forced tool (the tool trick)") and PARITY_PLAN card
  FND-09 (`structured<T>()`, "Anthropic tool trick") describe the old
  mechanism; update both to structured outputs.
