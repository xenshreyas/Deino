# 0028. Store due days as local noon, keep saved views as rows, and resolve extraction answers in Rust

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** AI-05 (Action items system)

## Context

FND-07 created `action_items` with `due_at` as an instant (`TEXT`, RFC
3339 UTC). AI-05 adds everything that reads it: the Action Items page with
"Overdue / Today / This week" filters and groups, a 09:00 reminder of what
is due today, and the `extract_action_items` stage (order 60), which turns
"by next Friday" into a due date. People say days, not instants, and they
mean their own local days. The page also needs saved views (named filters
shown as tabs, Circleback §8 "Views"), and the card left their storage
open: the settings document or a table.

LLMs are good at finding commitments and poor at calendar arithmetic and
at quoting identifiers exactly: a "next Friday" said on a Wednesday is
often the Friday two days later, and a cited transcript line may not
exist. Names come back as written in the transcript ("Jordan Elis"), not
as people.

## Decision

- **A due day is noon, local time, on that day.** The service converts a
  `NaiveDate` with the user's time zone (the new `deino_domain::LocalZone`
  port: `SystemZone` over the Mac's zone, `FixedZone` for tests) and stores
  the instant. Reading `due_at` in any zone within ±11 hours gives the same
  day back, so a trip does not move every due date by one. Filters are day
  windows computed in Rust (`ItemQuery::resolve`: overdue = before local
  midnight today, this week = today to next Monday's midnight, weeks run
  Monday to Sunday) and passed to SQL as UTC bounds, so the queries stay on
  their indexes.
- **Saved views are rows** in `action_item_views` (name unique ignoring
  case, a `position` for tab order, the `ActionItemFilter` as a JSON column
  that SQL never reads by field). Views are user data the page lists,
  renames and deletes one at a time, so each is a row that `DbChanged`
  refreshes; the settings document stays preferences only.
- **The model reads, Rust resolves.** The prompt asks only for clear
  commitments and returns `deino_llm::extraction::ActionItemsAnswer` (title,
  description, assignee as written, `due_phrase` and the model's own `due`,
  priority, `evidence_quote`, `evidence_segment_id` such as `s12`). Rust then
  resolves each field deterministically: `resolve_due` parses the phrase from
  the meeting's local day (weekday names, "next Friday", "end of the
  month", "the ninth of October", "in two weeks", "tomorrow") and falls back
  to the model's date only when plausible; `Roster` matches names to
  attendees, then to identified speakers, by whole name, contained words,
  address and one or two typos, leaving ambiguity unassigned; `Transcript`
  keeps a citation only when its line exists, moving it to the line that
  holds the quote when the quote is elsewhere.
- **Regeneration replaces only untouched AI items** (created by the AI,
  still open, `updated_at = created_at`), in one transaction, and skips new
  items that repeat a kept one (same evidence line or title).
- **The list query has three access paths** (`list_by_meeting.sql`,
  `list_by_person.sql`, `list_by_status.sql`) with the same parameters and
  residual filters; `ItemQuery::access_path` picks the one whose index
  narrows the rows most, and a test asserts each plan.

## Alternatives

- **Due dates as `TEXT` days (a new `due_on` column).** Exact, but the
  column is FND-07's and typed as an instant, INT-03's task tools sync
  instants, and the page would still need the zone for "today". Local noon
  keeps the schema and the instant semantics.
- **Due as local midnight or end of day.** Midnight reads as the previous
  day one hour west; 23:59 as the next day one hour east.
- **Saved views in the settings document.** No migration, but every
  rename rewrites the whole document, there is no `DbChanged`, and a list of
  user objects with IDs does not belong in preferences.
- **Trust the model's dates, names and line ids.** Simpler, and wrong often
  enough to break "open at this moment" and reminders.
- **One dynamic list query (`QueryBuilder`).** Flexible, but not checked at
  compile time (§3A), and its plans could not be asserted.

## Consequences

- Every caller that reads or writes a due day goes through `LocalZone`;
  ORG-02 (Home) and AI-08 (weekly roundup) reuse it and `TodayDigest`.
  EXP-01 had grown its own identical port in `deino_core::export`; that
  module now re-exports this one, so there is one zone port.
- The reminder fires from 09:00 until noon (a Mac asleep at 09:00 still
  gets it), once per day per app run; a restart inside that window can
  remind twice. Persisting the last reminded day would fix it if it matters.
- The phrase rules are English and conservative by design; new phrasings
  are added in `extraction/due.rs` with a test, and anything they miss
  falls back to the model's date.
- Changing a view's filter shape needs `#[serde(default)]` fields, as
  `ActionItemFilter` has, so stored views keep loading.
