/**
 * The ⌘F query language: free text plus `speaker:` tokens
 * (`speaker:priya budget`, `speaker:"Priya Shah"`). A token that names
 * exactly one speaker becomes a chip once it is complete (followed by a
 * space); one still being typed filters by name prefix as it stands.
 */
import type { SpeakerView } from '../types';

/** What the search bar holds: typed text and the speakers chosen as chips. */
export interface SearchQuery {
  text: string;
  /** `SpeakerView.key`s of the chips. */
  speakers: readonly string[];
}

/** No search. */
export const EMPTY_QUERY: SearchQuery = { text: '', speakers: [] };

/** Typed text split into the words to find and the speaker names typed. */
export interface ParsedText {
  words: string;
  names: string[];
}

const TOKEN = /(?:^|\s)speaker:(?:"([^"]*)"?|(\S*))/giu;
const COMPLETE_TOKEN = /(?:^|\s)speaker:(?:"([^"]+)"|([^\s"]+))(?=\s)/giu;

/** Splits `text` into its search words and its `speaker:` names (lower case). */
export function parseText(text: string): ParsedText {
  const names: string[] = [];
  const words = text.replace(TOKEN, (_match, quoted?: string, bare?: string) => {
    const name = (quoted ?? bare ?? '').trim().toLocaleLowerCase();
    if (name !== '') names.push(name);
    return ' ';
  });
  return { words: words.replace(/\s+/gu, ' ').trim(), names };
}

/** Whether a speaker called `name` answers to the typed `token` (lower case). */
export function nameMatches(name: string, token: string): boolean {
  const lower = name.toLocaleLowerCase();
  return lower.startsWith(token) || lower.split(/\s+/u).some((word) => word.startsWith(token));
}

/**
 * `query` with each complete `speaker:` token that names exactly one of
 * `speakers` turned into a chip; other tokens stay in the text.
 */
export function promoteTokens(query: SearchQuery, speakers: readonly SpeakerView[]): SearchQuery {
  const chips = [...query.speakers];
  const text = query.text.replace(COMPLETE_TOKEN, (match, quoted?: string, bare?: string) => {
    const token = (quoted ?? bare ?? '').toLocaleLowerCase();
    const named = speakers.filter((speaker) => nameMatches(speaker.name, token));
    const only = named.length === 1 ? named[0] : undefined;
    if (only === undefined) return match;
    if (!chips.includes(only.key)) chips.push(only.key);
    return match.startsWith(' ') ? ' ' : '';
  });
  return { text: text.replace(/^\s+/u, ''), speakers: chips };
}

/** `query` with the chip of `key` added, or removed when it is there. */
export function toggleSpeaker(query: SearchQuery, key: string): SearchQuery {
  const speakers = query.speakers.includes(key)
    ? query.speakers.filter((chip) => chip !== key)
    : [...query.speakers, key];
  return { ...query, speakers };
}

/** Whether `query` searches for anything. */
export function isActive(query: SearchQuery): boolean {
  return query.speakers.length > 0 || query.text.trim() !== '';
}
