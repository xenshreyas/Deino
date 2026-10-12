# 0045. Speaker tags from the meeting app's Accessibility tree, as segment-level priors

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** CAP-05 (Speaker names from the meeting app UI)

## Context

Granola names speakers on Zoom, Teams and Meet calls by reading the
meeting app's participant names and active-speaker indicator through the
OS Accessibility API (`docs/research/granola.md` §4). Deino records the
system track as one remote voice, diarizes it into "Speaker 1..N"
(CAP-03, ADR 0020) and names those by voiceprints, the calendar and
elimination (CAP-04, ADR 0029). The card asks for the window's names to be
strong priors that override voice matching, read every 250 ms while
recording, aligned with system-channel segments (a name wins a segment
lit for more than half of it), and to degrade silently.

Constraints: Deino must never prompt for Accessibility or change a system
setting by itself; polling must stay off the audio threads; each app's UI
changes without notice; and the rules must be testable without a call.

## Decision

- **A crate of its own, `deino-speaker-tags`** (heavy macOS-native code,
  like `deino-detect`): no `tauri`, no `deino-net`, depending only on
  `deino-domain`, whose new `CallApp`, `DisplayName` and `TagSpan` are the
  vocabulary shared with `deino-core`.
- **Poll, do not observe.** A watcher thread per recording reads the call
  window every 250 ms (`recv_timeout` on its stop channel is the timer).
  `AXObserver` notifications are not posted consistently for a tile's
  speaking state in any of the three apps, and a poll bounds the cost:
  100 ms per request, 3,000 elements per window, menus, scroll bars and
  text fields skipped.
- **One adapter per app behind `SpeakerAdapter`**, reading an `AxNode`
  snapshot so the same code runs on the live tree and on fixture dumps
  (`fixtures/`, a capture tool, a README). Zoom and Teams describe each
  tile and roster row as "Name (role), states…", read by one description
  scanner with per-app words; Meet in Chrome is read from Chromium's
  `AXDOMClassList` (a tile's `span.notranslate` name and its voice
  indicator's speaking classes, Meet's generated class names as Vexa's
  Meet speaker detection lists them, Apache-2.0), the lit indicator
  belonging to the nearest element that holds exactly one name. The user's own tile ("(me)", "(You)") never names a
  remote voice; crosstalk names nobody.
- **Chrome is asked to expose web content** with Chromium's
  `AXManualAccessibility` on its app element, once per process and only
  when a Meet room is open in one of its windows: what a
  screen reader does. Not `AXEnhancedUserInterface`, which OpenWhispr found
  breaks Chromium focus. Nothing else touches another app.
- **The recording's own clock.** `RecordingService::playhead()` gives the
  offset on the recording's timeline (host time since the track epoch,
  minus pauses), the same timeline segments are on; the watcher starts
  from a `StartObserver` (`StartObservers` fans out to ORG-03's link).
  Spans close halfway between polls (error ≤ 125 ms) and are appended to
  `speaker_tag_spans` every 10 s of recording time and at the end.
- **Priors at the segment level, inside `identify_speakers`.** Before
  voices are scored, `SpeakerTagPriors` aligns spans with the system
  track's segments (> 50 %), resolves each winning name to an invited
  attendee (full name, or a unique first name) or one exact person, and
  seats it: the person's speaker, the speaker already labelled with it,
  the diarized voice it wins more than half of, or a new speaker. Won
  segments move to their name's speaker; speakers left empty go. Speakers
  holding a name leave voice matching (named ones by person, labelled ones
  as claimed), so a window name always beats a voiceprint. Nothing ever
  moves off a speaker the user named, and the SQL re-checks it.
- **Names never create people.** An unresolved name becomes the
  speaker's label ("Alex Chen" instead of "Speaker 2"), which the
  transcript, the panel and the notes already show; renaming it in the
  Participants panel makes the person (and enrols the voice) as before.
- **Degrade without a sound.** No permission, no running app, no call
  window, an unknown layout or a failing read are typed `TagError`s,
  logged once at debug level; the poll names nobody and voices name that
  stretch. An unknown layout marks the app "Unsupported" in Settings until
  a window is recognised again.
- **Settings → Speaker tags** shows Enabled, Needs permission or
  Unsupported per app (`AXIsProcessTrusted`, never the prompting call),
  with "Open System Settings" for the Accessibility page.

## Alternatives

- **A Chrome extension for Meet (Granola's other path):** a second
  codebase and an install step; Chromium's own Accessibility covers it.
- **Screen capture and OCR of the tiles:** needs Screen Recording
  permission and far more CPU, and reads pixels instead of labels.
- **Naming whole diarized clusters by majority only:** without
  diarization (missing models) the system track is one "Them" speaker and
  a three-person call would get one name; the card's rule is per segment.
- **Creating a person for every name:** fills People with "iPhone" and
  "Conference Room"; a label shows the name without that side effect.
- **Live transcript naming (during the call):** the live lanes have one
  Them speaker; the post-meeting stage owns naming. The spans are already
  stored, so a later card can show them live.

## Consequences

- Fixture dumps pin the adapters' rules; the first set was modelled on
  each app's call window and must be replaced by real captures during the
  owner's manual QA (`crates/deino-speaker-tags/README.md`), then whenever
  an app changes its UI.
- Reading Chrome's web content costs Chrome and Deino some CPU during a
  call; measured in manual QA.
- A meeting re-identified later gets the same result: spans are kept, and
  rules 1 and 2 find the speakers an earlier run named or labelled.
- `deino-recording` gained `Playhead` and `StartObservers`, `deino-audio`
  `Recorder::epoch`; `Identification` gained a `Seating`.
