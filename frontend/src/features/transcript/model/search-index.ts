/**
 * In-transcript search (⌘F) over a memoised index of the rows.
 *
 * The index is built once per set of rows (the view memoises it), so a
 * keystroke only scans text: a case-insensitive, Unicode-aware match of
 * the typed words in every row whose speaker passes the filter. A
 * two-hour meeting (about 5,000 lines) searches in a few milliseconds
 * (`search-index.test.ts` holds it under 100 ms).
 */
import type { TextRange, TranscriptRow } from '../types';
import { nameMatches, parseText, type SearchQuery } from './search-query';

/** One row's searchable fields. */
interface IndexEntry {
  text: string;
  speakerKey: string;
  speakerName: string;
}

/** The rows, prepared for searching. */
export interface SearchIndex {
  entries: readonly IndexEntry[];
}

/** One match: a range of a row's text (the whole text for a speaker-only search). */
export interface SearchHit {
  row: number;
  range: TextRange;
  /** Position among all hits, from 0. */
  ordinal: number;
}

/** Every hit of a query, in transcript order, and the same hits by row. */
export interface SearchResult {
  hits: readonly SearchHit[];
  byRow: ReadonlyMap<number, readonly SearchHit[]>;
}

/** Nothing found (also the result of an empty query). */
export const NO_RESULT: SearchResult = { hits: [], byRow: new Map() };

/** Indexes `rows` for {@link searchTranscript}. */
export function buildSearchIndex(rows: readonly TranscriptRow[]): SearchIndex {
  return {
    entries: rows.map(({ line, speaker }) => ({
      text: line.text,
      speakerKey: speaker.key,
      speakerName: speaker.name,
    })),
  };
}

/** `text` with every regular-expression character escaped. */
function escape(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
}

/** Which rows the speaker chips and typed `speaker:` names allow. */
function speakerFilter(query: SearchQuery, names: readonly string[]) {
  if (query.speakers.length === 0 && names.length === 0) return () => true;
  const keys = new Set(query.speakers);
  return (entry: IndexEntry) =>
    keys.has(entry.speakerKey) || names.some((name) => nameMatches(entry.speakerName, name));
}

/** The ranges of `entry` that match `pattern`, or all of it without one. */
function rangesOf(entry: IndexEntry, pattern: RegExp | null): TextRange[] {
  if (pattern === null) return [{ start: 0, end: entry.text.length }];
  const ranges: TextRange[] = [];
  for (const match of entry.text.matchAll(pattern)) {
    ranges.push({ start: match.index, end: match.index + match[0].length });
  }
  return ranges;
}

/** Every hit of `query` in `index`, in order. */
export function searchTranscript(index: SearchIndex, query: SearchQuery): SearchResult {
  const { words, names } = parseText(query.text);
  if (words === '' && names.length === 0 && query.speakers.length === 0) return NO_RESULT;
  const pattern = words === '' ? null : new RegExp(escape(words), 'giu');
  const allowed = speakerFilter(query, names);
  const hits: SearchHit[] = [];
  const byRow = new Map<number, SearchHit[]>();
  index.entries.forEach((entry, row) => {
    if (!allowed(entry)) return;
    const found = rangesOf(entry, pattern).map((range, offset) => ({
      row,
      range,
      ordinal: hits.length + offset,
    }));
    if (found.length === 0) return;
    hits.push(...found);
    byRow.set(row, found);
  });
  return { hits, byRow };
}
