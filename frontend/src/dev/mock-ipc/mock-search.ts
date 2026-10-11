/**
 * A small stand-in for `deino_core::search` over the fixture meetings, for
 * `pnpm dev:mock`: every word must match (the last one as a prefix), hits
 * rank by how many words matched and where, and snippets carry highlight
 * ranges. It is not BM25 and has no stemming; it only has to look and
 * behave like the real thing.
 */
import type { MeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/search';

import type { FixtureMeeting } from './fixture-schema';

/** One searchable thing, like a `search_documents` row. */
export interface MockDocument {
  kind: Generated.HitKind;
  refId: string;
  meetingId: MeetingId | null;
  tsMs: number | null;
  title: string;
  body: string;
  date: string | null;
}

/** The documents of one fixture meeting: its title and its transcript. */
export function fixtureDocuments(fixture: FixtureMeeting, meetingId: MeetingId): MockDocument[] {
  const meeting = { meetingId, date: fixture.startedAt };
  const title: MockDocument = {
    kind: 'meeting',
    refId: meetingId,
    tsMs: null,
    title: fixture.title,
    body: '',
    ...meeting,
  };
  const moments = fixture.segments.map((segment, index): MockDocument => ({
    kind: 'segment',
    refId: `${meetingId}/${String(index)}`,
    tsMs: segment.startMs,
    title: '',
    body: segment.text,
    ...meeting,
  }));
  return [title, ...moments];
}

/** The query's words, lower case. */
export function queryWords(text: string): string[] {
  return text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((word) => word.length > 0);
}

/** `text` with each word that starts with a query word marked. */
export function mark(text: string, words: readonly string[]): Generated.Snippet {
  const highlights: Generated.Highlight[] = [];
  for (const match of text.matchAll(/[\p{L}\p{N}]+/gu)) {
    const word = match[0].toLowerCase();
    if (words.some((query) => word.startsWith(query))) {
      highlights.push({ start: match.index, end: match.index + match[0].length });
    }
  }
  return { text, highlights };
}

/** Whether every query word starts some word of `text`. */
export function matchesAll(text: string, words: readonly string[]): boolean {
  const own = queryWords(text);
  return words.every((query) => own.some((word) => word.startsWith(query)));
}

/** A matching document as a hit, or `null`. */
export function toHit(document: MockDocument, words: readonly string[], meetingTitle: string) {
  const inTitle = matchesAll(document.title, words);
  if (!inTitle && !matchesAll(document.body, words)) return null;
  const hit: Generated.Hit = {
    kind: document.kind,
    refId: document.refId,
    meetingId: document.meetingId,
    tsMs: document.tsMs,
    endMs: null,
    label:
      document.title === '' ? { text: meetingTitle, highlights: [] } : mark(document.title, words),
    snippet: mark(document.body, words),
    date: document.date,
    score: inTitle ? 4 : 1,
  };
  return hit;
}
