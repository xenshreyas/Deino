# 0036. Answer insights with a run-time schema, type every value in Rust, and run them as pipeline requests

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** AI-06 (Custom AI insights)

## Context

AI-06 lets the user define their own questions of meetings (Circleback
§10.3): a name, a description that is the prompt, optional typed fields
(text, number, yes/no, date, one of a list), how many answers to keep (Auto
or N), and when the insight runs (conditions shared with AUT-01, or only
when asked). Answers have the shape `[{value, speaker, timestamp_ms}]`,
show on the meeting's Insights tab and in one table per insight across
meetings (sortable, CSV), and can be run on past meetings in bulk.

Three forces shape the design. The answer's shape is the user's, so it is
not known at compile time, yet the gateway's `structured::<T>()` derives its
schema from a Rust type. Models return loose values (`"$40k"`, `"yes"`,
`"green"`) and invent line ids. And "Run on past meetings" is long-running
LLM work that must survive restarts, retry rate limits and show progress,
which §3.7 says is the pipeline's job.

## Decision

**A schema built per insight, typed wire values, Rust validation.**
`Gateway::structured_as::<T>(route, prompt, JsonOutput)` sends a schema
built at run time (`insights::schema`: one property per field with its JSON
type, enum options as an `enum`, `null` allowed, `maxItems` for a fixed
count) and parses the answer into a typed wire struct (`WireAnswer`, with
`WireScalar = Bool | Number | Text | List`), with the same repair retry as
`structured`. `validate` then checks every value against its field's type:
close values are repaired (currency and `k`/`M` suffixes, yes/no words,
written dates, options matched ignoring case and punctuation), others are
dropped; answers with nothing valid, and repeats, are dropped; Auto keeps at
most 30, a fixed count N. The stored and returned value is the typed
`InsightValue::{Text, Fields}` with `FieldValue::{Text, Number, Boolean,
Date, Choice}`; nothing outside the gateway adapter sees JSON.

**Attribution is resolved in Rust.** The prompt shows the transcript with
NOT-02's references (`[s12 04:31] Speaker: text`, `MaterialSources`); each
answer cites `evidence: "s12"`. `Attribution` resolves the first reference
that names a real line to its segment, start time and speaker. Answers store
the segment ID and the speaker as named; reads replace the speaker with the
segment's current name (`SpeakerNames`), so renames (CAP-04) reach insights.

**Two tables.** `insight_defs` holds a definition (fields and conditions as
typed JSON documents, read whole) with a `revision` bumped by every edit of
what it asks. `insight_results` holds one row per (insight, meeting): the
latest run's `status` (`pending`, `done`, `failed` with a user-safe error),
its answers as a write-once JSON snapshot, and the revision they answer, so
the UI can say "answers an earlier version". Earlier answers are kept while
a new run is pending or after it failed.

**Runs are pipeline requests.** The `insights` stage (order 85: after tags
and folders settle, which conditions read, and before AUT-01's
`automations` at 95) runs the meeting's requested runs (pending or failed
rows) when there are any, and otherwise marks every matching automatic
insight pending and runs those. "Run on past meetings" and the tab's "Run"
insert pending rows and re-run the meeting's `insights` stage through
`PipelineService::rerun` (`RunScheduler` port, implemented in the app over
Tauri state because the pipeline is built after the service). Pending rows
are the progress the page shows; `DbChanged` refreshes it. A rate limit
leaves the row pending for the runner's retry; on the last attempt, or for
any other failure, the error is stored on the row and the other insights
still run.

## Alternatives

- **A fixed generic schema** (`fields: [{name, value: string}]`) with
  `structured::<T>()`. No gateway change, but the provider could not hold
  the model to the field names, types or options, so more answers would be
  repaired or dropped, and the built-in model's grammar would not constrain
  them at all.
- **`serde_json::Value` answers through the core.** Simplest to write and
  forbidden by §3A; every consumer (tab, table, CSV, exports, AUT-01) would
  re-check types.
- **One row per answer.** Queryable by SQL, but a run's answers are written
  and replaced together, the status of a run with no answers needs a row
  anyway, and nothing filters answers by field in SQL.
- **A background task per bulk run.** Faster to start, but it would
  duplicate the pipeline's durability, retries, cancellation and progress,
  and §3.7 forbids ad-hoc background work.

## Consequences

- AUT-01 reuses `deino_domain::conditions` (adding its conditions there)
  and can read `insight_results` for its "generate insight" step.
- Re-running the whole pipeline on a meeting regenerates its matching
  automatic insights; running one insight re-runs only that one.
- A bulk run moves each past meeting through `processing` while its stage
  runs, as any re-run does.
- Long transcripts go to the model whole (as AI-05's do); chunking for the
  local model is a follow-up.
- Exports render an Insights section in every format, the account export
  gains `insights.csv`, and the table's CSV goes through
  `deino_export::csv::insight_table`.
