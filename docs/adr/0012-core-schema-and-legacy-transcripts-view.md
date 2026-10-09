# 0012. Create the core schema in one migration and keep legacy transcripts readable through a view

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** FND-07 (`docs/references/architecture.md` §5 and (b) FND-07)

## Context

PARITY_PLAN §4.1 assigns FND-07 every core table: transcript segments,
speakers, people, companies, attendees, calendar accounts and events,
folders, tags and action items, plus lifecycle columns on `meetings`. Later
cards (FND-05b, ORG-01..07, AI-05, TRN-01) build on them in parallel, so the
shape has to be settled at once. Three questions needed an answer:

1. **Legacy transcripts.** The inherited code reads and writes the old
   `transcripts` table in seven places across four modules owned by other
   cards: `api_save_transcript` (FND-05b replaces it), import (CAP-11),
   re-transcription (CAP-08), search, delete and the meeting page's paging
   (TRN-01, ORG-07). The card says `transcript_segments` replaces
   `transcripts`, and existing meetings must still open.
2. **The meeting <-> calendar link.** §4.1 lists both
   `meetings.calendar_event_id` and `calendar_events.meeting_id`.
3. **`meetings` itself** predates `STRICT` and is referenced by four legacy
   tables with `ON DELETE CASCADE`.

## Decision

- **One migration**, `20261009060435_fnd07_core_schema.sql`, with the rules
  of §3A: `STRICT` tables, `CHECK`ed enums mirrored by `deino-domain` enums,
  `NOT NULL` unless unknown is meaningful, explicit `ON DELETE`, an index for
  every foreign key and list order (asserted by `EXPLAIN QUERY PLAN` tests),
  and no `WITHOUT ROWID` tables, which SQLite's update hook (and so
  `DbChanged`) cannot see. Integrity a foreign key cannot express is checked
  where it is cheapest: a trigger keeps a segment's speaker in its meeting;
  `FolderRepo::move_folder` refuses folder cycles (SQLite forbids recursive
  queries in triggers).
- **Legacy transcripts move, and a view keeps the old shape.** Every
  `transcripts` row is copied into `transcript_segments` (`channel = 'mixed'`,
  because legacy capture mixed before transcription; the UUID of
  `transcript-<uuid>` IDs is kept), the table is dropped, and `transcripts`
  becomes a view with `INSTEAD OF INSERT` and `INSTEAD OF DELETE` triggers.
  The inherited code keeps working without a single edit, its writes land in
  the one source of truth, and they reach the UI as `transcript_segments`
  changes. `timestamp`, formerly a wall-clock string, is now the recording
  offset (`HH:MM:SS`), which is what the UI shows anyway.
- **One direction for links.** A meeting points at its event
  (`meetings.calendar_event_id`, `ON DELETE SET NULL`, indexed); events do not
  point back, so the two can never disagree. "The meeting of this event" is
  an indexed lookup on that column.
- **`meetings` is extended, not rebuilt.** New columns are added with
  `ALTER TABLE` (nullable, or `NOT NULL` with a default: existing meetings
  become `ready` and `live`). Rebuilding the table would cascade-delete every
  child row, and a migration cannot turn foreign keys off inside sqlx's
  transaction. `MeetingId` stays opaque text, so every existing meeting opens.
- **Repositories** live in `deino-core`, one module per aggregate, each with
  small ports (for example `SegmentReader`/`SegmentWriter`), a SQLite
  implementation and a fake that pass the same contract tests.

## Alternatives

- **Rewrite the legacy reads and writes to use `transcript_segments`.** Seven
  call sites in modules that FND-05b, CAP-08, CAP-11 and TRN-01 are about to
  replace; it would duplicate the legacy-row mapping three times and collide
  with those cards. The view puts the mapping in one place and costs nothing
  to delete later.
- **Keep `transcripts` as a table and mirror it into `transcript_segments`
  with triggers.** Two copies of every transcript, and two sources of truth
  for edits.
- **Store both `calendar_event_id` and `calendar_events.meeting_id`.** A
  denormalised pair with no single writer; one of them would drift.
- **Rebuild `meetings` as `STRICT` with `CHECK`s on every column.** Needs
  foreign keys off, which sqlx's per-migration transaction does not allow
  without a `-- no-transaction` migration that can fail half-applied.

## Consequences

- Legacy code needs no changes now, and the cards that port it delete the
  view and its two triggers in their own migration once nothing uses them
  (follow-up in PARITY_PLAN §7.2).
- The view hides soft-deleted segments; a legacy `DELETE` therefore leaves
  segments a user soft-deleted (TRN-02) in place. Nothing soft-deletes before
  the legacy callers are gone.
- `meetings` keeps its non-`STRICT` affinity rules for the legacy columns;
  the new columns still have `CHECK`s.
- The `Table` set changed: `transcripts` left (a view never reaches the update
  hook) and every core table joined, which regenerated the TypeScript bindings.
