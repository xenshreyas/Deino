# 0046. The trash hides meetings at read time; a merge keeps its parts

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** ORG-07 (Meetings list, views, bulk actions, trash, merge)

## Context

Before ORG-07, deleting a meeting was the inherited `api_delete_meeting`: a
hard delete of the row (foreign keys cascading to everything else) that left
the recording and the note images on disk. ORG-07 asks for three things that
shape the data model:

1. **A trash** (Granola §3.6): deleted meetings wait 30 days, then they and
   their audio are gone; restoring must bring back *everything*: transcript,
   notes, enhanced notes, tags, folders, people, action items, insights.
2. **Merging** two or more meetings (Meetily community request #393):
   concatenate the transcripts with time offsets, unite the attendees, then
   regenerate.
3. Trashing a meeting while its post-meeting pipeline runs, and a merge
   whose search embeddings now cover only the earliest part.

## Decision

**A soft delete, filtered at read time.** `meetings.deleted_at` is the whole
trash. Nothing else changes when a meeting is trashed, so restoring is one
`UPDATE` and everything is back by construction. Every reader skips trashed
rows: `MeetingRepo::find` (and therefore the meeting page, exports, the CLI
and pipeline stages), the list query, search (`search_documents` stays
indexed and is filtered at query time, ADR 0032), entities' counts and
"last met", related meetings. "Delete forever" and the purge (`TrashPurger`,
every 6 hours, rows trashed over 30 days ago) delete the row first, in one
statement whose foreign keys cascade, and only then the files under
`recordings/<id>` and `meetings/<id>`: a crash in between leaves orphaned
files, never a row pointing at missing audio. The sample meeting's action
items are demo data and go with it; a real meeting's outlive it
(`ON DELETE SET NULL`).

**A merge keeps its parts.** The earliest meeting absorbs the others in one
transaction: segments and word timings shift by each part's offset on a
single timeline, speakers join by person (or the user), attendance keeps
the stronger role (as ORG-04's person merge does), tags, folders, action
items and voiceprints move, and the notes combine under one heading per
part. Recordings are not re-encoded: `meeting_merge_parts` records each
absorbed meeting's title, day, offset, length and recording paths, so the
audio stays reachable (and the purge deletes it), and the absorbed rows are
deleted.

**The pipeline through a port.** `deino-core` cannot depend on
`deino-pipeline`, so the trash and the merge call a small `MeetingWork`
port: `cancel` (trashing stops the meeting's running stage) and `reindex`
(a merge re-runs the merged meeting's `index` stage). The app implements it
over `PipelineService`, looked up when called because the pipeline is
composed after the Meetings page's services. Both are best effort and
logged: the trash or the merge has already committed.

## Alternatives

- **Move trashed meetings to shadow tables.** Restore would have to copy
  every child table back and keep up with every card that adds one. Rejected:
  the read-time filter needs only the readers to agree, and they share
  `MeetingRepo::find`.
- **Delete child rows on trash and keep a JSON snapshot to restore.** Breaks
  foreign keys from other tables (action items, voiceprints) and the "restores
  everything" acceptance.
- **Concatenate the audio on merge.** Re-encoding hours of two-track audio
  is slow and lossy, and the CMAF writer (ADR 0014) owns those files. Parts
  keep the originals; the player reading them is TRN-02's follow-up.
- **Let a merged meeting wait for its next pipeline run.** Its embeddings
  would miss the absorbed transcripts until then; the `index` stage is cheap
  to re-run alone.

## Consequences

- Every new reader of meetings must skip `deleted_at IS NOT NULL`, or go
  through `MeetingRepo::find`. Reviewers check it.
- A cancelled stage stays cancelled after a restore; the user re-runs it from
  the meeting's status strip.
- The player does not yet play a merged meeting's later parts
  (`meeting_merge_parts`); PARITY_PLAN §7.2 tracks it for TRN-02.
