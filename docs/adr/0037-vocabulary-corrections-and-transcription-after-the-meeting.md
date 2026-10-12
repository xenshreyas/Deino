# 0037. Correct transcripts with the vocabulary after decoding, and transcribe record-only meetings in the pipeline

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** TRN-05 (custom vocabulary and corrections), CAP-08 (recording-only
  mode and post-pass transcription); `docs/references/audio-stt.md` (finding
  5, CAP-08 recommendations), `docs/references/spike-stt-streaming.md` §4

## Context

TRN-05 asks for the user's jargon and names (people, companies, invitees)
to come out right, live and after the meeting, and for every correction to
be undoable. The engine cannot help: transcribe-cpp 0.3.1 sets
`TRANSCRIBE_FEATURE_VOCABULARY` only for Whisper, Qwen3-ASR, Granite,
FunASR-Nano and MOSS; Parakeet's `capabilities.cpp` never does, so
`RunOptions::vocabulary` is ignored (checked in the vendored
`transcribe-cpp-sys-0.3.1/src/arch/*/capabilities.cpp`).

CAP-08 asks for a mode that records audio only (the CPU stays under 15 %)
and transcribes after stop, and for an optional high-accuracy re-pass that
replaces the live transcript while keeping speakers. Batch decoding of a
long clip can stop emitting text partway (the spike's LibriVox case).

## Decision

**Vocabulary (TRN-05).**

- **Corrections after decoding.** A `Corrector` over a `Lexicon` of the
  user's entries and every known name. Matching works on whole-word *cores*
  (punctuation and possessives kept outside), case-insensitively; exact
  phrases first (a term's spellings, a replacement's phrases, and known
  names, which are kept as heard so nothing fuzzy rewrites them), then names
  by Jaro-Winkler on their letters: 0.92 for names of several words (the
  card), 0.95 for one word and only when the heard word is capitalised, has
  at least five letters, shares the name's first letter, is within two
  letters of its length and is not a prefix of it (or it of the word). The
  guards keep "hunted", "Christians" and "General Motor teams" out of
  "Hunter", "Christian" and "General Motors" (property and unit tests).
- **Two passes.** Live, the recorder corrects each row it saves and each
  segment it shows (decorators around the router's store and events, so the
  router and its echo holdback are untouched), with the user's entries and
  all people and companies. After the meeting, `vocab_fixups` (order 40)
  corrects again with the meeting's invitees and newer entries. It runs
  after `final_transcription` (20) and after diarization (30, 35) splits
  segments, so originals are kept per final segment, and before
  `enhance_notes` (50). Segments the user edited are never touched.
- **Undo through kept originals.** `vocab_corrections` keeps the engine's
  text and words per corrected segment (the first original survives later
  corrections). The stage writes it in the same transaction as the segment,
  with a compare-and-set on the text; the live pass right after each saved
  batch. Undo restores the original only while the segment still reads the
  corrected text, and stamps `undone_at` so the stage never redoes it.
- **Notes.** The user's terms feed NOT-02's `Vocabulary` port.

**Transcription after the meeting (CAP-08).**

- **Record-only skips the live path entirely.** With live transcription off
  the recorder opens no engine (no worker process, no model) and starts no
  live pipeline (no 16 kHz resampling, AEC or VAD); the writers' placed
  audio ring simply fills and drops, which costs nothing.
- **A stage, not the recorder.** `final_transcription` (order 20) decodes
  the finished `mic.m4a`/`system.m4a` with `ExtAudioFile` (priming trimmed),
  finds utterances with the lanes' earshot VAD, cuts them to at most 30 s,
  and runs the pool's batch worker on each; an utterance that returns fewer
  than one word per second over ten seconds is read again in halves. The
  transcript replaces the old one in one transaction, each utterance taking
  the speaker the old segments gave its time (`carried_speaker`), else the
  channel's (the microphone is Me; system audio is the lone remote speaker).
- **Re-pass policy.** Off by default. It never replaces a transcript the user
  edited; a missing model skips it at once and an engine failure is retried,
  then skipped, so a live transcript always reaches the notes.

## Alternatives

- **Hotwords in the engine.** Not available for Parakeet; switching the
  default engine to Whisper for its prompt biasing contradicts CAP-01.
- **Correct only after the meeting.** Simpler, but the live transcript and
  the record screen would show the misspelling the user already fixed.
- **Single-word fuzzy matching at 0.92 without guards.** Turns common words
  into names ("same" → "Sam", "hunted" → "Hunter").
- **Keep originals in a column of `transcript_segments`.** Belongs to
  FND-07, and the original must survive across the stage and undo; a TRN-05
  table with a cascade keeps the schema's owner boundaries.
- **Run the live pipeline with idle lanes when record-only.** Cheaper to
  write, but the AEC and resampling would still run for nothing.
- **Transcribe after stop inside the recorder.** It would block stop, skip
  the pipeline's retries and progress, and be lost on a crash.

## Consequences

- Live and post-meeting transcripts spell names and jargon the user's way;
  every correction is listed in the meeting's Details panel and undoable.
- A segment that diarization split after the live pass loses the undo of
  that live correction (its text no longer matches); the stage's own
  corrections are made after the split and keep theirs.
- Record-only meetings get their transcript, speakers and notes from the
  pipeline a minute or so after stop; the record screen shows no live text.
- The re-pass replaces segment IDs, so citations in notes written before a
  manual re-run point at removed segments (the normal order writes notes
  after it).
- Revisit when transcribe-cpp gains Parakeet biasing (pass the lexicon's
  terms through `RunOptions::vocabulary` and keep the fixups for names).
