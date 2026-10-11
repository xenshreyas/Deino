# 0033. Search the open transcript in the UI, over virtual rows of one line each

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** TRN-01 (transcript view v2); `docs/references/ui.md` (Transcript bubble),
  `docs/research/granola.md` §4, `docs/research/circleback.md` §7.1

## Context

TRN-01 asks for a transcript that stays smooth with 5,000 segments, searches a
two-hour meeting in under 100 ms (⌘F, a `speaker:` filter, "n of N",
Enter/⇧Enter), follows live lines without jank, and opens at a linked moment
(`?t=`, and `?segment=` for NOT-03's citations). The inherited views paged the transcript through the legacy
`api_get_meeting_transcripts` command and kept only the loaded pages, so
search could not see the whole meeting; the recorder's live view rendered
every line. Three questions needed an answer: where the in-meeting search
runs, what a virtual row is, and who owns the transcript read.

## Decision

- **One read, one plugin.** `deino_core::transcript::TranscriptService`
  returns the whole saved transcript of a meeting (segments in spoken order,
  each with its speaker ID) as `plugin:transcript|get`. The UI names the
  speakers from CAP-04's `plugin:speakers|participants` (the same names,
  colours and "Who is …?" as the Participants panel) and overlays the
  recorder's `LiveSegment` events. The recorder's own `transcript` command
  goes. A two-hour meeting is a few thousand rows (one to a few megabytes of
  JSON with word timings), read once and refreshed by `DbChanged`, so no
  paging.
- **Search runs in the UI** over a memoised index of the rows
  (`model/search-index.ts`): one entry per line (text, speaker key and name),
  rebuilt only when the rows change; a keystroke compiles one Unicode,
  case-insensitive pattern of the typed words and scans the entries that pass
  the speaker filter. 5,000 lines search in 3–7 ms in Chrome and well under
  100 ms in happy-dom (`search-index.test.ts`, `e2e/transcript.spec.ts`).
  `speaker:name ` becomes a chip once it names exactly one speaker.
- **A virtual row is one line (bubble)**, keyed by segment ID and measured as
  it renders (`@tanstack/react-virtual`); a turn's first row carries the
  speaker header. A live line that grows keeps its key, so only its row
  re-measures; the virtualiser corrects rows above the viewport.
- **Live following is a `ResizeObserver` on the list's sizer**, which pins the
  scroll to the end while the user was at the bottom, so new rows and growing
  lines are followed without effects; a scroll away shows "Jump to now".

## Alternatives

- **Search in SQLite (FTS5, ORG-08's `search_fts`).** Right for searching
  across meetings, but for the open meeting it adds a round trip per
  keystroke, cannot see live lines that are not saved yet, and needs
  character offsets for highlighting that FTS does not return. Rejected for
  the in-meeting find; ORG-08 links its hits here with `?t=`.
- **A Web Worker for search.** Unnecessary at these sizes (single-digit
  milliseconds on the main thread) and it would copy the transcript into
  the worker on every live update. Revisit if a meeting grows past about
  50,000 lines.
- **One virtual row per speaker turn** (`ui.md` suggests keying by turn). A
  monologue turn can be hundreds of lines tall, which defeats virtualisation
  and makes a search hit or a moment land on a row larger than the viewport.
  Rows per line keep the DOM at about 30 rows whatever the turn length.
- **Paging the transcript** (the legacy view). Keeps memory flat but makes
  search, "Jump to now" and `?t=` depend on what has been loaded.

## Consequences

- The meeting page and the record screen render the same view;
  `VirtualizedTranscriptView`, `TranscriptView`, `TranscriptPanel` and the
  recorder's `LiveTranscript` are gone.
- Bubble actions are a registry (`features/transcript/actions.ts`): TRN-02
  (delete), TRN-03 (play from here), TRN-04 (clip) and TRN-06 (comment) fill
  their hidden entries. Speaker naming stays CAP-04's: a turn's chip opens
  its `AssignSpeaker` for that line.
- TRN-02's edits belong in the `transcript` plugin and service.
- The whole transcript lives in the query cache while a meeting is open; at
  two hours that is a few megabytes, an acceptable cost for instant search.
  If word timings make it heavy, `get` can leave them out until TRN-03
  needs them.
