# 0029. Name speakers by mutual-best voiceprint matches, and enrol only from confirmed renames

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** CAP-04 (Speaker naming and voice profiles)

## Context

CAP-03 (ADR 0020) leaves every meeting with "Speaker 1..N" rows, each with
its WeSpeaker ResNet34-LM centroid (`speakers.centroid`). CAP-04 must name
them: from voice profiles the user built by naming people before, from
the calendar, and by asking when unsure. The diarization spike
(`docs/references/spike-diarization.md`) measured the rules on synthetic
voices and decided the numbers: same speaker 0.90 at the speaker level,
different speakers at most 0.66, an unknown voice 0.35–0.38. It also
recorded two failures that shape the design:

- a phantom cluster contaminated with another voice matched the wrong
  person at **0.81**; only a margin over the runner-up **and** mutual best
  together rejected it;
- a wrong enrolment (two voices merged) caused false accepts in every later
  meeting.

anarlog keeps up to five exemplars per person and scores a person by the
best of them; OpenWhispr updates one profile vector by an exponential
moving average (`0.3 · new + 0.7 · old`).

The matching primitives (`pick_unique`, `complete_by_elimination`) shipped
in `deino-diarize`, which depends on `deino-core`, while the stage's rules
live in `deino-core` (ADR 0020's split).

## Decision

- **Voiceprints are exemplars, not an average.** `voiceprints` stores up to
  five 256-d centroids per person (`embedding BLOB` of 1024 bytes, the
  model's ID, the source meeting, `quality`); enrolling a sixth evicts the
  lowest quality, oldest first, in the same transaction
  (`VoiceprintRepo::enrol`). A person's score for a voice is the **max**
  over their exemplars (`best_score`). Quality is the speaker's clean,
  overlap-free speech (`select_speaker_spans`, up to twelve 10 s spans)
  over one minute.
- **Only a confirmed "rename all segments" enrols**, which includes "Yes"
  on a suggestion. Renaming one segment, merging, automatic naming and
  elimination never enrol. A voice with under 3 s of clean speech is not
  enrolled (it is a phantom's worth).
- **`identify_speakers` (order 35)** reads the meeting
  (`diarization::identify::Lineup`) and applies pure rules
  (`identify::decide`, generic over the keys, so the spike's cases are
  table tests):
  1. the unnamed microphone speaker is the `people.is_me` person;
  2. **by voice**: the best score ≥ `autoAssign` (0.72), ≥
     `autoAssignNonAttendee` (0.77) for someone not invited, a margin ≥
     `margin` (0.08) over every other candidate, and mutual best
     (`pick_unique`: the person has no closer voice);
  3. **by elimination**: when the track holds exactly as many speakers as
     attendees expected on it and all but one of each are paired, the last
     speaker is the last attendee (no voiceprint needed: a one-on-one call
     names itself);
  4. **suggestions**: the remaining best candidate scoring ≥ `suggest`
     (0.55), each person suggested once, to the closest voice. A score
     above 0.72 that failed the margin or mutual best is suggested too: the
     user decides, nothing is assumed.

  Candidates exclude people already named in the meeting and, on a call,
  the user. Voices with under 3 s of speech are neither named by voice nor
  suggested (phantom pruning on top of CAP-03's). Speakers the user named
  are never renamed. The thresholds live in `Settings::speakers` (with an
  on/off switch) so they can be calibrated on real meetings.
- **Known-voice merging stays in the engine.** `TrackRequest::known`
  carries the expected attendees' exemplars (the user's only when the
  microphone itself is diarized); `OnnxTrackDiarizer` passes them as
  `KnownSpeaker`s, and anarlog's `apply_known_speakers` merges clusters of
  one known voice under the calendar's speaker floor. Merging stored
  speakers after the fact by voice would merge a contaminated phantom into
  the wrong person, the failure above.
- **The matching primitives move to `deino-core::voiceprints`** (with the
  enrolment spans), where the rules that use them live; the engine's known
  merge imports them from there. They are pure functions with no native
  dependency.
- **Names propagate by reference.** Speakers point at people, so the
  transcript, the panel and the notes input (`legacy_summary` prefixes each
  line with the name) read the current name. A rename moves the meeting's
  action items from the previous person to the new one unless that person
  still speaks in the meeting; the UI then offers to regenerate the notes.
  Suggestions are a table (`speaker_suggestions`, one per speaker), not
  columns on `speakers`, so CAP-03's rows stay as they were.

## Alternatives

- **One averaged profile per person (OpenWhispr's EMA):** one bad sample
  moves the profile for good, and a voice heard in two rooms averages into
  neither. The max over five exemplars keeps each condition and bounds the
  damage of one bad sample to that sample, which Settings can delete.
- **Enrolling from automatic or elimination matches:** grows profiles
  faster but compounds errors; the spike saw exactly that.
- **Merging speakers in `identify_speakers` when two match the same
  person:** rejected for the phantom case; the engine's merge works on
  window embeddings under the speaker-count floor instead.
- **A threshold only (no margin, no mutual best):** accepted the 0.81
  phantom in the spike.
- **A port to keep the matching in `deino-diarize`:** a trait for pure
  arithmetic, and the rule tests would have to live with the native
  engine.

## Consequences

- One meeting's confirmed name makes the next meeting with that voice name
  itself: covered on fixtures (`deino-pipeline`'s `voice_profile_tests`,
  scripted centroids) and on the real models and fixture audio
  (`crates/deino-diarize/tests/voice_profiles.rs`, ignored by default:
  the standup names Maya after the one-on-one did).
- The thresholds are synthetic-voice numbers. Real cross-meeting scores run
  lower; calibrate `Settings::speakers.thresholds` on dogfood meetings and
  re-tune before changing the defaults.
- Merging people (ORG-04) must move their voiceprints and suggestions with
  them; deleting a person deletes their voiceprints (`ON DELETE CASCADE`).
- A new embedding model leaves old voiceprints unused (they are filtered by
  `embedding_model`) until people are renamed again.
