/** Pure helpers for showing and editing vocabulary entries. */
import type { TermInput, VocabKind, VocabTerm } from './types';

/** A comma-separated field as its phrases, trimmed, blanks dropped. */
export function phrases(text: string): string[] {
  return text
    .split(',')
    .map((phrase) => phrase.trim().replace(/\s+/g, ' '))
    .filter((phrase) => phrase.length > 0);
}

/** What the composer's two fields mean for `kind`. */
export function termInput(kind: VocabKind, spelled: string, soundsLike: string): TermInput {
  return { kind, text: spelled.trim(), soundsLike: phrases(soundsLike) };
}

/** The entries split by kind, each in the order Rust lists them. */
export function byKind(entries: readonly VocabTerm[]): {
  terms: VocabTerm[];
  replacements: VocabTerm[];
} {
  return {
    terms: entries.filter((entry) => entry.kind === 'term'),
    replacements: entries.filter((entry) => entry.kind === 'replacement'),
  };
}

/**
 * The words a correction changed: `original` and `corrected` without the
 * words they share at either end ("Thanks Sreyas." → "Thanks Shreyas."
 * changed `Sreyas.` to `Shreyas.`).
 */
export function changedWords(original: string, corrected: string): { from: string; to: string } {
  const before = original.split(/\s+/).filter(Boolean);
  const after = corrected.split(/\s+/).filter(Boolean);
  let head = 0;
  while (head < before.length && head < after.length && before[head] === after[head]) head += 1;
  let tail = 0;
  const fits = () => tail < before.length - head && tail < after.length - head;
  while (fits() && before.at(-1 - tail) === after.at(-1 - tail)) tail += 1;
  return {
    from: before.slice(head, before.length - tail).join(' '),
    to: after.slice(head, after.length - tail).join(' '),
  };
}
