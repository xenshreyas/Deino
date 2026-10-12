# 0047. Pre-meeting briefs from local data, on a fingerprinted schedule

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** NOT-08 (scope cut by §8 D12 and D13)

## Context

Granola writes a two-to-three-bullet brief overnight for each upcoming
external meeting, cites every fact, and shows nothing when there is too
little to say (`docs/research/granola.md` §7.1). Its sources include the
web and Gmail. The owner parked the chat agent (AI-02, the web tool) and the
email integrations (INT-07), so Deino's briefs can only read what is on the
Mac. The brief has to be ready before the meeting without the UI open, must
not call the model again for nothing, and must survive restarts and sleep.

## Decision

- **Sources are local only:** the attendees as people (the user left out),
  their companies, the people's and companies' AI summaries (ORG-04),
  earlier meetings with them (`EntityReads` digests: the newest summary,
  else the user's notes), the series' last meeting (ORG-03's
  `RelatedService`, through a new `related_to(RelationKey)` that takes an
  event's key), their open action items (AI-05) and the invitation's
  description. Every text goes through NOT-09's `TextRedactor` (the
  `SettingRedactor` in the app) before the prompt is built.
- **Two "no brief" gates.** A deterministic check runs first: at least one
  earlier meeting with content, or two other facts (items, summaries); the
  invitation never counts. A new contact therefore costs no model call.
  Then the model may answer `insufficient: true`, and fewer than two bullets
  with a valid citation also mean no brief.
- **Structured output and citations like ADR 0030.** Route `brief` (smart
  tier) returns `BriefAnswer { insufficient, bullets[{ text, sources }] }`
  through `Gateway::structured`. Sources are shown as `[m1]`, `[a1]`,
  `[p1]`, `[c1]`, `[e1]` blocks with absolute dates; `SourceIndex` resolves
  references (`m1`, `[M1]`, `m1 (Kickoff)`), drops unknown ones, keeps
  three per bullet and four bullets. Citations are stored typed: meeting,
  moment (an item's source segment), action item, person, company, event,
  each with its label, without foreign keys.
- **One `briefs` row per calendar event**, with `status` (`ready` or
  `insufficient`), `bullets_json` (a write-once snapshot), `read_at` and
  `feedback`. `inputs_hash` is the SHA-256 of the rendered system and user
  prompt: a run whose fingerprint matches the stored one writes nothing and
  calls nothing. Regenerating clears `read_at` and `feedback`.
- **The schedule is pure rules plus a ticker.** `due_trigger` says whether
  the 06:00 run (events later the same local day, from 06:00 on, so a Mac
  asleep at six catches up) or the hour-before run is due; `qualifies`
  picks external meetings, or every meeting with two or more attendees
  (`Settings::briefs`). `BriefScheduler::tick` runs each due trigger once
  per process; the app ticks it every minute like the action item reminder.
  Restarts are safe because the fingerprint, not the in-memory record,
  decides whether anything is written.

## Alternatives

- **A pipeline stage per event.** The pipeline (ADR 0018) is keyed on
  meetings and runs after them; an event may have no meeting yet, and
  creating one to get a job would litter the library.
- **A durable run table per trigger.** It would record what the fingerprint
  already implies; with the fingerprint, re-running a trigger is free.
- **Recomputing on read.** Opening the note would wait for the model, and
  the 1-minute notification (ORG-02) needs the brief ready.
- **Free-text answers with inline references.** Harder to validate; the
  structured answer makes "insufficient" explicit and citations a list.

## Consequences

- ORG-02 reads `BriefService::for_event` and drops in `EventBrief` or the
  presentational `BriefCard`; meeting notes linked to an event show the
  brief above the notes (`MeetingBrief`).
- Web and email sources (AI-02, INT-07) are later work: each adds source
  blocks with a new reference letter; the fingerprint then covers them.
- A prompt wording change regenerates every upcoming brief once, at its
  next trigger.
- The thumbs down is stored and not yet used (PARITY_PLAN §7.2).
