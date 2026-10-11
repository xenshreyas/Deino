/**
 * Where each timed word sits in a line's text, so the view can highlight the
 * word being played and seek to the word clicked (Circleback §7.1).
 *
 * Words are found in order, each after the previous one. If one is missing
 * (the user edited the line, or the engine's tokens do not spell the text),
 * the line has no word ranges and plays and seeks by the line instead.
 */
import type { TextRange, WordTiming } from '../types';

/** A word: where it is in the text, and when it was said. */
export interface TimedWord {
  /** Its characters in the line's text. */
  range: TextRange;
  /** Milliseconds from the start of the recording. */
  start: number;
  /** Milliseconds from the start of the recording. */
  end: number;
}

/** The words of `text` with their times, or `null` when `words` do not spell it. */
export function timedWords(text: string, words: readonly WordTiming[]): TimedWord[] | null {
  if (words.length === 0) return null;
  const found: TimedWord[] = [];
  let cursor = 0;
  for (const word of words) {
    const token = word.text.trim();
    if (token === '') continue;
    const at = text.indexOf(token, cursor);
    if (at < 0) return null;
    found.push({ range: { start: at, end: at + token.length }, start: word.start, end: word.end });
    cursor = at + token.length;
  }
  return found;
}

/** The index of the word said at `ms`: the last that started by then, or -1. */
export function wordAt(words: readonly TimedWord[], ms: number): number {
  let current = -1;
  for (const [index, word] of words.entries()) {
    if (word.start > ms) break;
    current = index;
  }
  return current;
}
