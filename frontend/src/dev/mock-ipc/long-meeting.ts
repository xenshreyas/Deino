/**
 * A very long generated meeting for performance checks of the transcript
 * view (TRN-01: smooth with 5,000 segments, search under 100 ms).
 *
 * Off by default, so the fixture meetings stay the only data. A test (or a
 * person) turns it on for one tab by putting the segment count in session
 * storage before the app loads:
 *
 *     sessionStorage.setItem('deino.mock.longMeeting', '5000')
 *
 * The text is deterministic (a fixed seed), and every 250th line mentions
 * the "quarterly roadmap", so a search has a known number of hits.
 */
import type { Participant, PersonId } from '@/lib/ipc/generated/speakers';
import type * as Generated from '@/lib/ipc/generated/transcript';

import {
  LONG_MEETING_ID as LONG_MEETING_KEYED_ID,
  LONG_MEETING_KEY,
  PHRASE,
  PHRASE_EVERY,
} from './long-meeting-contract';

export {
  LONG_MEETING_KEY,
  LONG_MEETING_TITLE,
  PHRASE,
  PHRASE_EVERY,
} from './long-meeting-contract';

/** The generated meeting's ID. */
export const LONG_MEETING_ID = LONG_MEETING_KEYED_ID as Generated.MeetingId;

/** How long the meeting runs: two hours. */
const DURATION_MS = 2 * 60 * 60 * 1000;

const WORDS = (
  'we should ship the beta next week and then look at onboarding metrics before the ' +
  'launch review because the team wants clear numbers on retention churn pricing and ' +
  'support load so let us agree owners dates and a short list of risks today'
).split(' ');

/** A voice of the meeting: the user, then three colleagues. */
export interface LongMeetingVoice {
  speakerId: Generated.SpeakerId;
  name: string;
  isMe: boolean;
  color: number;
}

const SPEAKERS: readonly LongMeetingVoice[] = [
  { speakerId: 'speaker-long-me' as Generated.SpeakerId, name: 'Me', isMe: true, color: 1 },
  ...['Priya Shah', 'Marcus Lee', 'Ana Ruiz'].map((name, index) => ({
    speakerId: `speaker-long-${String(index)}` as Generated.SpeakerId,
    name,
    isMe: false,
    color: index + 2,
  })),
];

/** The meeting's voices, for the mock speakers (`mock-speaker-store.ts`). */
export function longMeetingVoices(): readonly LongMeetingVoice[] {
  return SPEAKERS;
}

/** The voices as participants, each named after a person: for tests without the mock store. */
export function longMeetingParticipants(): Participant[] {
  return SPEAKERS.map((voice, index) => ({
    speakerId: voice.speakerId,
    label: `Speaker ${String(index + 1)}`,
    color: voice.color,
    isMe: voice.isMe,
    person: { id: `person-long-${String(index)}` as PersonId, name: voice.name },
    speech: 0,
    sharePercent: 0,
    suggestion: null,
  }));
}

/** The requested segment count, or `null` when the meeting is off. */
export function requestedLongMeeting(): number | null {
  try {
    const value = Number.parseInt(sessionStorage.getItem(LONG_MEETING_KEY) ?? '', 10);
    return Number.isInteger(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

/** A small deterministic generator (an LCG), so every run has the same text. */
function generator(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1_103_515_245 + 12_345) % 2 ** 31;
    return state / 2 ** 31;
  };
}

/** Line `index` of `count`: who says it, and what. */
function lineAt(index: number, count: number, random: () => number): Generated.Segment {
  const speaker = SPEAKERS[Math.floor(random() * SPEAKERS.length)] ?? SPEAKERS[0];
  const length = 6 + Math.floor(random() * 26);
  const words = Array.from({ length }, () => WORDS[Math.floor(random() * WORDS.length)] ?? 'and');
  if (index % PHRASE_EVERY === 0) words.splice(2, 0, 'the', PHRASE);
  const text = `${words.join(' ')}.`;
  const start = Math.floor((index * DURATION_MS) / count);
  return {
    id: `segment-long-${String(index).padStart(5, '0')}` as Generated.SegmentId,
    meetingId: LONG_MEETING_ID,
    channel: speaker?.isMe === true ? 'mic' : 'system',
    speakerId: speaker?.speakerId ?? null,
    start,
    end: start + Math.floor(DURATION_MS / count) - 200,
    text: text.charAt(0).toUpperCase() + text.slice(1),
    words: [],
    isDeleted: false,
    editedAt: null,
    originalText: null,
  };
}

/** The meeting's transcript with `count` lines over two hours. */
export function longTranscript(count: number): Generated.Transcript {
  const random = generator(count);
  const segments = Array.from({ length: count }, (_, index) => lineAt(index, count, random));
  return { segments };
}
