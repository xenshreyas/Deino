#!/usr/bin/env node
/**
 * Synthesizes the fixture audio of one meeting with macOS `say`.
 *
 *   node fixtures/tools/synthesize-audio.mjs fixtures/meetings/standup.json
 *
 * Writes `fixtures/audio/<slug>/{mic,system}.m4a`: mono AAC at 16 kHz, one
 * track per capture channel, covering the segments that end within the first
 * {@link CLIP_LIMIT_MS}. Every segment is spoken inside its own
 * `[startMs, endMs)` slot on its own track (the rate is fitted to the slot),
 * so the audio is aligned with the transcript, and the other track is silent
 * meanwhile. It then records the clip in the meeting's `audio` field.
 *
 * Run `retime.mjs` first; it owns the timings. Needs macOS (`say`, `afconvert`).
 */
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative } from 'node:path';

import { formatMeeting } from './format.mjs';

/** Clips stay under a minute (the fixture size budget). */
const CLIP_LIMIT_MS = 55_000;
/** Silence kept after the last spoken segment. */
const CLIP_TAIL_MS = 500;
const SAMPLE_RATE = 16_000;
const AAC_BITRATE = 24_000;
/** One voice per person, so the two tracks and the speakers sound distinct. */
const VOICES = {
  'shreyas@deino.dev': 'Rishi',
  'maya@deino.dev': 'Samantha',
  'diego@deino.dev': 'Daniel',
  'priya@deino.dev': 'Moira',
  'tom@deino.dev': 'Karen',
};
const FALLBACK_VOICE = 'Fred';

/** Renders `text` with `say` into 16-bit PCM samples. */
function speak(text, voice, wordsPerMinute, scratch) {
  const file = join(scratch, 'segment.wav');
  const options = ['-v', voice, '-r', String(Math.round(wordsPerMinute)), '-o', file];
  execFileSync('say', [...options, '--file-format=WAVE', `--data-format=LEI16@${SAMPLE_RATE}`, text]);
  return pcmOf(readFileSync(file));
}

/** The samples of a WAVE file's `data` chunk. */
function pcmOf(wave) {
  let offset = 12;
  while (offset + 8 <= wave.length) {
    const id = wave.toString('ascii', offset, offset + 4);
    const size = wave.readUInt32LE(offset + 4);
    if (id === 'data') {
      const data = wave.subarray(offset + 8, offset + 8 + size);
      return new Int16Array(data.buffer.slice(data.byteOffset, data.byteOffset + data.length));
    }
    offset += 8 + size + (size % 2);
  }
  throw new Error('say produced a WAVE file without data');
}

/** Speaks `segment` so it fits its slot, speeding up until it does. */
function fitToSlot(segment, voice, scratch) {
  const slot = Math.floor(((segment.endMs - segment.startMs) * SAMPLE_RATE) / 1000);
  let rate = (segment.words.length / (segment.endMs - segment.startMs)) * 60_000;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const samples = speak(segment.text, voice, rate, scratch);
    if (samples.length <= slot) return samples;
    rate *= (samples.length / slot) * 1.04;
  }
  throw new Error(`${segment.text.slice(0, 30)}… does not fit its slot`);
}

/** A 16-bit mono WAVE file holding `samples`. */
function waveOf(samples) {
  const header = Buffer.alloc(44);
  header.write('RIFF', 0, 'ascii');
  header.writeUInt32LE(36 + samples.byteLength, 4);
  header.write('WAVEfmt ', 8, 'ascii');
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(1, 22);
  header.writeUInt32LE(SAMPLE_RATE, 24);
  header.writeUInt32LE(SAMPLE_RATE * 2, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write('data', 36, 'ascii');
  header.writeUInt32LE(samples.byteLength, 40);
  return Buffer.concat([header, Buffer.from(samples.buffer)]);
}

/** Encodes `samples` to AAC in an MPEG-4 audio file at `path`. */
function encode(samples, path, scratch) {
  const wave = join(scratch, 'track.wav');
  writeFileSync(wave, waveOf(samples));
  mkdirSync(dirname(path), { recursive: true });
  execFileSync('afconvert', ['-f', 'm4af', '-d', 'aac', '-b', String(AAC_BITRATE), wave, path]);
}

const meetingPath = process.argv[2];
if (meetingPath === undefined) throw new Error('usage: synthesize-audio.mjs <meeting.json>');
const meeting = JSON.parse(readFileSync(meetingPath, 'utf8'));
const fixturesRoot = dirname(dirname(meetingPath));
const slug = basename(meetingPath, '.json');
const spoken = meeting.segments.filter((segment) => segment.endMs <= CLIP_LIMIT_MS);
const clipEndMs = (spoken.at(-1)?.endMs ?? 0) + CLIP_TAIL_MS;
const tracks = {
  mic: new Int16Array((clipEndMs * SAMPLE_RATE) / 1000),
  system: new Int16Array((clipEndMs * SAMPLE_RATE) / 1000),
};
const voiceOf = new Map(meeting.speakers.map((s) => [s.label, VOICES[s.attendee] ?? FALLBACK_VOICE]));
const scratch = mkdtempSync(join(tmpdir(), 'deino-fixture-audio-'));
try {
  for (const segment of spoken) {
    const samples = fitToSlot(segment, voiceOf.get(segment.speaker), scratch);
    tracks[segment.channel].set(samples, Math.floor((segment.startMs * SAMPLE_RATE) / 1000));
  }
  const audio = { mic: `audio/${slug}/mic.m4a`, system: `audio/${slug}/system.m4a`, clipEndMs };
  encode(tracks.mic, join(fixturesRoot, audio.mic), scratch);
  encode(tracks.system, join(fixturesRoot, audio.system), scratch);
  meeting.audio = audio;
  writeFileSync(meetingPath, formatMeeting(meeting));
  console.log(`${relative('.', meetingPath)}: ${spoken.length} segments, ${clipEndMs} ms of audio`);
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
