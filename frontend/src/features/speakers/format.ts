/**
 * How speakers read: their display name, initials for the avatar, their
 * talk time, and their colour slot's utilities.
 */
import { speakersCopy } from './copy';
import type { Participant } from './types';

/** A participant's name: the person they are, "You" for an unnamed
 * microphone, else their label ("Speaker 2"). */
export function displayName(participant: Participant): string {
  if (participant.person !== null) return participant.person.name;
  return participant.isMe ? speakersCopy.you : participant.label;
}

/** Splits text into user-perceived characters, so "Émile" starts with "É". */
const graphemes = new Intl.Segmenter('en', { granularity: 'grapheme' });

/** The first user-perceived character of `word`. */
function firstCharacter(word: string): string {
  for (const { segment } of graphemes.segment(word)) return segment;
  return '';
}

/** Up to two initials: "Maya Chen" → "MC", "Speaker 2" → "2", "You" → "Y". */
export function initials(name: string): string {
  const speakerNumber = /^Speaker (\d+)$/u.exec(name.trim());
  if (speakerNumber?.[1] !== undefined) return speakerNumber[1];
  const words = name
    .trim()
    .split(/\s+/u)
    .filter((word) => word !== '');
  return words.slice(0, 2).map(firstCharacter).join('').toUpperCase();
}

/** Milliseconds as "12m 04s", "48s" or "1h 02m". */
export function talkTime(ms: number): string {
  const seconds = Math.round(ms / 1000);
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  const pad = (value: number) => String(value).padStart(2, '0');
  if (hours > 0) return `${hours}h ${pad(minutes)}m`;
  if (minutes > 0) return `${minutes}m ${pad(rest)}s`;
  return `${rest}s`;
}

/** The utilities of one `--speaker-N` slot, as literal classes Tailwind keeps. */
export interface SpeakerTone {
  /** Text in the speaker colour. */
  text: string;
  /** A solid fill (the talk-time bar). */
  fill: string;
  /** The soft avatar background, the colour at 16%. */
  soft: string;
}

const TONES: readonly SpeakerTone[] = [
  { text: 'text-speaker-1', fill: 'bg-speaker-1', soft: 'bg-speaker-1/16' },
  { text: 'text-speaker-2', fill: 'bg-speaker-2', soft: 'bg-speaker-2/16' },
  { text: 'text-speaker-3', fill: 'bg-speaker-3', soft: 'bg-speaker-3/16' },
  { text: 'text-speaker-4', fill: 'bg-speaker-4', soft: 'bg-speaker-4/16' },
  { text: 'text-speaker-5', fill: 'bg-speaker-5', soft: 'bg-speaker-5/16' },
  { text: 'text-speaker-6', fill: 'bg-speaker-6', soft: 'bg-speaker-6/16' },
];

/** The tone of palette slot `color` (1–6); unknown slots cycle. */
export function speakerTone(color: number): SpeakerTone {
  const index = (((color - 1) % TONES.length) + TONES.length) % TONES.length;
  const tone = TONES[index] ?? TONES[0];
  if (tone === undefined) throw new Error('The speaker palette is empty.');
  return tone;
}
