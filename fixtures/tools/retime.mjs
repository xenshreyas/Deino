#!/usr/bin/env node
/**
 * Computes the timing of fixture meetings from their text, in place.
 *
 *   node fixtures/tools/retime.mjs fixtures/meetings/*.json
 *
 * The JSON files are the single source of truth. Writers edit `segments[].text`
 * (and `speaker`); this tool then fills in everything that follows from the
 * text, deterministically:
 *
 * - `channel`: the speaker's channel;
 * - `startMs`/`endMs` and `words[]`: a speaking-rate model (about 150 words a
 *   minute, longer words take longer, punctuation adds a pause);
 * - `durationMs`: the last segment's end plus a short tail.
 *
 * Silences are preserved: a segment keeps the gap it already has before it,
 * so re-running after a text edit only shifts what follows. A new segment
 * may carry `pauseMs` for a deliberate silence (a screen share, a break);
 * the tool turns it into the gap and drops the field.
 */
import { readFileSync, writeFileSync } from 'node:fs';

import { formatMeeting, ordered } from './format.mjs';

/** Fixed cost of any word, in ms. */
const WORD_BASE_MS = 120;
/** Extra time per letter or digit. */
const PER_CHARACTER_MS = 55;
/** Bounds of one word's duration. */
const WORD_MIN_MS = 160;
const WORD_MAX_MS = 900;
/** Pauses after punctuation inside a segment. */
const PUNCTUATION_PAUSE_MS = { ',': 180, ';': 220, ':': 220, '.': 320, '?': 320, '!': 320 };
/** Default silence before a turn by a new speaker, and within one speaker's turn. */
const TURN_GAP_MS = [350, 900];
const SAME_SPEAKER_GAP_MS = [150, 450];
/** Silence before the first word, and after the last. */
const LEAD_IN_MS = [600, 1800];
const TAIL_MS = [1500, 3000];

/** A stable pseudo-random integer in `[low, high)` for `key`. */
function jitter(key, [low, high]) {
  let hash = 2166136261;
  for (const character of key) {
    hash = Math.imul(hash ^ character.codePointAt(0), 16777619) >>> 0;
  }
  return low + (hash % (high - low));
}

/** How long one word takes to say, without the pause after it. */
function wordMs(word) {
  const letters = word.replace(/[^\p{L}\p{N}]/gu, '').length;
  return Math.min(WORD_MAX_MS, Math.max(WORD_MIN_MS, WORD_BASE_MS + PER_CHARACTER_MS * letters));
}

/** The pause after `word`, unless it ends the segment. */
function pauseAfterMs(word) {
  return PUNCTUATION_PAUSE_MS[word.at(-1)] ?? 0;
}

/** Times every word of `text` from `startMs`; returns the words and the end. */
function timeWords(text, startMs) {
  const tokens = text.split(/\s+/).filter(Boolean);
  let cursor = startMs;
  const words = tokens.map((token, index) => {
    const word = { text: token, startMs: cursor, endMs: cursor + wordMs(token) };
    cursor = word.endMs + (index < tokens.length - 1 ? pauseAfterMs(token) : 0);
    return word;
  });
  return { words, endMs: cursor };
}

/** The silence before segment `index`: kept, requested, or a default. */
function gapBefore(segments, index) {
  const segment = segments[index];
  const previous = segments[index - 1];
  if (segment.pauseMs !== undefined) return segment.pauseMs;
  if (previous === undefined) return segment.startMs ?? jitter(segment.text, LEAD_IN_MS);
  if (segment.startMs !== undefined && previous.endMs !== undefined) {
    return Math.max(0, segment.startMs - previous.endMs);
  }
  const range = previous.speaker === segment.speaker ? SAME_SPEAKER_GAP_MS : TURN_GAP_MS;
  return jitter(`${index}:${segment.text}`, range);
}

/** Returns `meeting` with channels, timings and duration recomputed. */
function retime(meeting) {
  const channels = new Map(meeting.speakers.map((speaker) => [speaker.label, speaker.channel]));
  const oldEnd = meeting.segments.at(-1)?.endMs;
  const tail =
    oldEnd !== undefined && meeting.durationMs !== undefined
      ? meeting.durationMs - oldEnd
      : jitter(meeting.id, TAIL_MS);
  const gaps = meeting.segments.map((_, index) => gapBefore(meeting.segments, index));
  let cursor = 0;
  const segments = meeting.segments.map((segment, index) => {
    const channel = channels.get(segment.speaker);
    if (channel === undefined) throw new Error(`${meeting.id}: unknown speaker ${segment.speaker}`);
    const startMs = cursor + gaps[index];
    const { words, endMs } = timeWords(segment.text, startMs);
    cursor = endMs;
    return { speaker: segment.speaker, channel, startMs, endMs, text: segment.text, words };
  });
  return { ...ordered(meeting), durationMs: cursor + tail, segments };
}

for (const path of process.argv.slice(2)) {
  const meeting = JSON.parse(readFileSync(path, 'utf8'));
  const retimed = retime(meeting);
  writeFileSync(path, formatMeeting(retimed));
  const minutes = (retimed.durationMs / 60000).toFixed(1);
  console.log(`${path}: ${retimed.segments.length} segments, ${minutes} min`);
}
