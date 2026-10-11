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
import type * as Generated from '@/lib/ipc/generated/transcript';

/** The session storage key that asks for the meeting. */
export const LONG_MEETING_KEY = 'deino.mock.longMeeting';

/** The generated meeting's ID and title. */
export const LONG_MEETING_ID = 'meeting-generated-long' as Generated.MeetingId;
export const LONG_MEETING_TITLE = 'Two-hour planning marathon';

/** Every this many lines mentions the search phrase. */
export const PHRASE_EVERY = 250;
export const PHRASE = 'quarterly roadmap';

/** How long the meeting runs: two hours. */
const DURATION_MS = 2 * 60 * 60 * 1000;

const WORDS = (
  'we should ship the beta next week and then look at onboarding metrics before the ' +
  'launch review because the team wants clear numbers on retention churn pricing and ' +
  'support load so let us agree owners dates and a short list of risks today'
).split(' ');

const SPEAKERS: readonly Generated.TranscriptSpeaker[] = [
  {
    id: 'speaker-long-me' as Generated.SpeakerId,
    name: 'Me',
    personId: null,
    isMe: true,
    color: 1,
  },
  ...['Priya Shah', 'Marcus Lee', 'Ana Ruiz'].map((name, index) => ({
    id: `speaker-long-${String(index)}` as Generated.SpeakerId,
    name,
    personId: `person-long-${String(index)}` as Generated.PersonId,
    isMe: false,
    color: index + 2,
  })),
];

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
    speakerId: speaker?.id ?? null,
    start,
    end: start + Math.floor(DURATION_MS / count) - 200,
    text: text.charAt(0).toUpperCase() + text.slice(1),
    words: [],
    isDeleted: false,
    editedAt: null,
  };
}

/** The meeting's transcript with `count` lines over two hours. */
export function longTranscript(count: number): Generated.Transcript {
  const random = generator(count);
  const segments = Array.from({ length: count }, (_, index) => lineAt(index, count, random));
  return { segments, speakers: [...SPEAKERS] };
}
