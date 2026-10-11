/**
 * The ⌘F search state of one transcript view: the query, its hits over a
 * memoised index of the rows, and the current hit. Moving to a hit calls
 * `reveal` with its row, so the list scrolls it into view.
 */
import { useMemo, useState } from 'react';

import {
  buildSearchIndex,
  type SearchHit,
  type SearchResult,
  searchTranscript,
} from '../model/search-index';
import { EMPTY_QUERY, promoteTokens, type SearchQuery, toggleSpeaker } from '../model/search-query';
import type { SpeakerView, TranscriptRow } from '../types';

/** What the search bar and the list read and do. */
export interface TranscriptSearch {
  query: SearchQuery;
  result: SearchResult;
  /** The current hit, if there are any. */
  current: SearchHit | undefined;
  setText: (text: string) => void;
  toggleSpeaker: (key: string) => void;
  /** Removes the last chip (Backspace in an empty field). */
  popSpeaker: () => void;
  next: () => void;
  previous: () => void;
  clear: () => void;
}

/** The query edits, each starting a new search through `search`. */
function queryActions(
  query: SearchQuery,
  speakers: readonly SpeakerView[],
  search: (next: SearchQuery) => void,
) {
  return {
    setText: (text: string) => {
      search(promoteTokens({ ...query, text }, speakers));
    },
    toggleSpeaker: (key: string) => {
      search(toggleSpeaker(query, key));
    },
    popSpeaker: () => {
      search({ ...query, speakers: query.speakers.slice(0, -1) });
    },
    clear: () => {
      search(EMPTY_QUERY);
    },
  };
}

/** Search over `rows`, whose speakers are `speakers`. */
export function useTranscriptSearch(
  rows: readonly TranscriptRow[],
  speakers: readonly SpeakerView[],
  reveal: (row: number) => void,
): TranscriptSearch {
  const [query, setQuery] = useState<SearchQuery>(EMPTY_QUERY);
  const [cursor, setCursor] = useState(0);
  const index = useMemo(() => buildSearchIndex(rows), [rows]);
  const result = useMemo(() => searchTranscript(index, query), [index, query]);
  const count = result.hits.length;
  const current = count === 0 ? undefined : result.hits[Math.min(cursor, count - 1)];

  const search = (next: SearchQuery) => {
    setQuery(next);
    setCursor(0);
    const first = searchTranscript(index, next).hits[0];
    if (first !== undefined) reveal(first.row);
  };
  const step = (delta: number) => {
    if (current === undefined) return;
    const target = (current.ordinal + delta + count) % count;
    setCursor(target);
    const hit = result.hits[target];
    if (hit !== undefined) reveal(hit.row);
  };
  return {
    query,
    result,
    current,
    ...queryActions(query, speakers, search),
    next: () => {
      step(1);
    },
    previous: () => {
      step(-1);
    },
  };
}
