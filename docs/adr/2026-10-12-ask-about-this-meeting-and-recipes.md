# "Ask about this meeting" streams cited text; recipes are rows synced from a library

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** AI-04, with AI-02-lite (PARITY_PLAN D13)

## Context

D13 parks the cross-meeting chat agent (AI-02) and asks for a lightweight
"Ask about this meeting" panel first: single-meeting questions with
transcript citations, a short thread per meeting, and recipes (AI-04)
scoped to one meeting. Answers should stream, like Granola's chat, and
cite like the enhanced notes (NOT-03). The notes engine (ADR 0030) gets
its citations from a structured answer, but a structured answer cannot
stream readable text, and a chat answer has no fixed shape.

## Decision

**Answers stream as text with `[s12]` markers; Rust resolves them at the end.**

- `deino_core::ask` reads the meeting through `enhance::MaterialSources`
  (the transcript redacted by NOT-09's `TextRedactor`) and passes the
  user's notes and the newest enhanced notes through the same redactor.
  The prompt shows the transcript with ADR 0030's short references
  (`[s12 04:31] Maya: …`) and asks for `[s12]` or `[s12, s14]` after each
  statement. Route `chat` (no new route, no `llm_calls` migration).
- `Gateway::stream` feeds a `TextSink`; the app forwards each piece over
  the request's `tauri::ipc::Channel<AskStreamEvent>` (§4.0). The UI hides
  markers while streaming.
- On the last piece, `ask::parse_answer` turns the text into
  `AnswerPart { text, citations }` runs: each marker resolves through
  `SegmentIndex`; references to lines that do not exist are dropped with
  their marker; brackets that are not markers stay text. Every stored
  citation therefore names a real segment of this meeting. The UI opens a
  run's citations with NOT-03's excerpts (`notes::CitedExcerpts`).
- **Thread:** `ask_messages`, one row per question or answer (`parts_json`,
  the recipe's slug and name as a label), pruned to the newest 40 per
  meeting in the same transaction as each append. The prompt carries the
  last 6 messages as plain text without citations, so recipes chain.
  Questions are saved only with their answer; a failed call saves nothing.
- **Recipes:** a `recipes` table (`slug` unique, `name`, `description`,
  `prompt`, `kind` single|multi, `built_in`). The 20 built-ins live in
  `recipes::LIBRARY` and are upserted by slug on first use each run
  (changed rows only; a user's recipe holding the slug wins; retired
  built-ins removed). Built-ins are read-only; Remix copies one. User
  recipes are always `single`; the three cross-meeting built-ins are
  `multi`, listed as WIP and refused by `ask`, `preview` and Remix.
- **Sharing:** a recipes file (`"format": "deino.recipes"`, version 1) and
  the share string `deino://recipe?data=<base64url(JSON)>` of name,
  description, prompt and kind. Only the string is produced and parsed
  here; opening the link is DEV-05's.

## Alternatives

- **Structured answers** (`{ text, evidence[] }` per paragraph, as the
  notes do): validated by construction, but nothing readable streams
  until the JSON closes, and the model's prose suffers.
- **Tool calls for citations**: providers stream tool input poorly and the
  built-in model has no tools; markers work on every provider.
- **AI-02's `chat_threads`/`chat_messages` tables now**: their scope model
  (folder, person, all) is AI-02's to design; a per-meeting table keeps
  this card small and is easy to migrate into them later.
- **Built-ins only in code, the table for the user's recipes**: simpler,
  but the card asks for one `recipes` table with `built_in`, and rows give
  every recipe an ID the thread and the share string can use alike.

## Consequences

- The model may cite a wrong but real line; the eval
  (`crates/deino-llm/tests/llm_eval/ask.rs`) scores whether the answer cites
  the fact's evidence line or a neighbour.
- A transcript beyond 200,000 characters is cut with a note; long meetings
  need retrieval or map-reduce (AI-02).
- Multi-meeting recipes, the model picker per recipe (`model_route`) and
  recipe deep links wait for AI-02 and DEV-05.
