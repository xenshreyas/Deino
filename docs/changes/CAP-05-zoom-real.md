# CAP-05 · Speaker tags on real Zoom

## Progress log (PARITY_PLAN §7.1)
Owner QA: a real 42-minute Zoom call saved 0 speaker tags ("the call
window's layout was not recognised"). Real captures (`deino
--dump-call-window zoom`) show why: Zoom Workplace (macOS, October 2026)
has **no talking indicator** anywhere, and its tiles are `AXTabGroup`s
described "Name, Computer audio muted|unmuted[, Video off]". The adapter's
hand-modelled grammar ("…, Talking" on `AXGroup`s) never matched.

- **`AxNode::frame`** (`Frame`: `AXPosition` + `AXSize`, whole points),
  optional in the JSON so older dumps still read. The live reader reads it
  for `AXTabGroup` only (`FRAMED_ROLES`); the full read for every element.
- **Zoom adapter** (`adapters/zoom.rs`, `adapters/zoom/tiles.rs`): the call
  window is titled "Zoom Meeting" and has at least one tile (a titled
  window without tiles is an unrecognised layout). The user's own name
  comes from the Zoom Workplace window's account button ("Zoom, Name, In a
  Zoom Meeting, Pro account"), else a Participants row marked "(me)". The
  reading's participants are the tiles minus the user; the speaker is
  Speaker View's large tile (more than 4× every other tile's area; the
  user's own large tile is `Active::Me`), else the one unmuted remote,
  else nobody (`Nobody`, or `Several` when more than one is unmuted).
- **Context windows.** `SpeakerAdapter::is_context_window` and
  `read_with(window, context)` (defaults: none, `read`): the watch reads
  an app's context windows (Zoom Workplace) with its call windows and
  passes them in; only call windows ready Chrome or count as calls.
- **Roster spans.** The watch keeps who each call window listed as spans
  (`Presence`, one `Timeline` per participant) and saves them through
  `TagSink::save_roster` (`Outbox` holds both kinds until saved). New
  table `speaker_tag_roster` (migration `20261019170000_cap05_speaker_tag_roster`),
  `SpeakerTagWriter::append_roster`, `SpeakerTagReader::roster`; the app's
  `DatabaseSink` writes it.
- **The 1:1 rule** (`deino_core::speaker_tags::one_to_one`): when the
  roster lists exactly one remote person for at least 60 s in all (shorter
  joins are ignored), that person wins every system-track segment, so the
  seats merge the over-split diarized speakers into one named speaker
  (segments on a speaker the user named stay). Otherwise the spans vote as
  before. `SpeakerTagPriors` now plans when there are spans or a roster.
- **Fixtures.** New anonymised real captures:
  `zoom/real-speaker-view-remote-talking.json` (tile frames added by hand
  from the screen-reading tool: the capture had none),
  `zoom/real-gallery-remote-unmuted.json`, `zoom/real-silent-all-muted.json`.
  Removed the hand-modelled `zoom/gallery-bea-talking`, `gallery-me-talking`
  and `gallery-silent` (Zoom never shows that format). Kept
  `gallery-alex-talking` and `screen-share-no-indicator` only for
  deino-consent's chat tests; the Zoom adapter now reads them as an
  unrecognised layout.
- **Tests.** Tile parsing, roster, self exclusion (account button, "(me)"
  row), Speaker View, the one-unmuted pick, nobody/several
  (`zoom/tiles_tests.rs`); the real dumps (`tests/adapters.rs`); roster
  spans (`presence.rs`, `watch_tests.rs`); the repo round trip and index;
  `one_to_one` (brief joins, split presence, two people); `identify_speakers`
  merging a three-way split into the one remote person, and a group call
  still voting by spans and naming the rest by elimination; the end-to-end
  fixture test (`deino-core/tests/speaker_tags_fixtures.rs`) on the real
  dumps.

## Follow-ups (PARITY_PLAN §7.2)
- Owner QA: a real Zoom group call in Speaker View, to confirm the large
  tile follows the speaker and the 4× ratio holds at other window sizes.
- Owner QA: capture a real Speaker View dump with frames (the new reader
  keeps them) and replace the hand-added frames.
- deino-consent: its Zoom chat dumps are still hand-modelled; the real call
  window showed no meeting controls (hidden until hovered). Capture the
  real chat panel and controls before relying on the Zoom chat notice.
- Teams and Meet adapters are still modelled by hand; capture them the same
  way.

## Architecture (ARCHITECTURE.md)
- Speaker tags: "Zoom has no talking indicator: the adapter reads tile
  sizes (Speaker View) and audio states, and the user from the Zoom
  Workplace window. The watch also records the remote roster
  (`speaker_tag_roster`); `identify_speakers` names every remote voice of a
  one-to-one call after the one remote person."
- Tables: add `speaker_tag_roster`.

## CLAUDE.md
- `crates/deino-speaker-tags` row: "Zoom by tile size and audio (no talking
  indicator), the remote roster as spans".
- `crates/deino-core` row, `speaker_tags`: "the one-to-one rule over the
  roster".
