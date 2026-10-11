/**
 * The virtual list behind the transcript (`@tanstack/react-virtual`, rows
 * measured as they render and keyed by segment ID, so a growing live line
 * keeps its row) and live following.
 *
 * While live, the list follows the newest line as long as the user is at
 * the bottom: a `ResizeObserver` on the list's sizer sees every growth
 * (new rows, a line growing in place) and pins the scroll to the end. A
 * scroll away from the bottom stops following, and `jumpToNow` resumes it.
 * Rows above the viewport that change size are corrected by the
 * virtualiser, so nothing on screen jumps.
 */
import { useVirtualizer, type Virtualizer } from '@tanstack/react-virtual';
import { type RefObject, useCallback, useRef, useState } from 'react';

import type { TranscriptRow } from '../types';
import { estimateRowHeight } from '../model/row-height';

/** Within this many pixels of the end counts as "at the bottom". */
const FOLLOW_SLACK_PX = 32;

/** Rows rendered beyond the viewport on each side. */
const OVERSCAN = 8;

/** Space above the first row and below the last (room for "Jump to now"). */
const PADDING_START_PX = 16;
const PADDING_END_PX = 72;

/** What the list component and its owners use. */
export interface TranscriptListController {
  scrollRef: RefObject<HTMLDivElement>;
  /** Attach to the element sized to the list's total height. */
  sizerRef: (node: HTMLDivElement | null) => void;
  virtualizer: Virtualizer<HTMLDivElement, Element>;
  /** Live and pinned to the newest line. */
  following: boolean;
  onScroll: () => void;
  /** Scrolls `row` to the middle of the view (search hits, moments). */
  reveal: (row: number) => void;
  /** Scrolls to the newest line and follows it again. */
  jumpToNow: () => void;
}

/** Pins `element` to its end. */
function pinToEnd(element: HTMLElement | null): void {
  if (element !== null) element.scrollTop = element.scrollHeight;
}

/** Keeps the scroll at the end while `following` holds, as the content grows. */
function useFollowObserver(scrollRef: RefObject<HTMLDivElement>, following: () => boolean) {
  const observer = useRef<ResizeObserver | null>(null);
  return useCallback(
    (node: HTMLDivElement | null) => {
      observer.current?.disconnect();
      observer.current = null;
      if (node === null || typeof ResizeObserver === 'undefined') return;
      observer.current = new ResizeObserver(() => {
        if (following()) pinToEnd(scrollRef.current);
      });
      observer.current.observe(node);
    },
    [scrollRef, following],
  );
}

/**
 * Whether the list follows the newest line: state for the pill, and a ref
 * the resize observer reads between renders.
 */
function useFollowing(live: boolean) {
  const followingRef = useRef(live);
  const [following, setFollowing] = useState(live);
  return {
    following: following && live,
    followingRef,
    isFollowing: useCallback(() => live && followingRef.current, [live]),
    follow: (next: boolean) => {
      followingRef.current = next;
      setFollowing(next);
    },
  };
}

/** The virtualiser over `rows`; a live list opens at its end. */
function useRowVirtualizer(
  rows: readonly TranscriptRow[],
  live: boolean,
  scrollRef: RefObject<HTMLDivElement>,
) {
  return useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => estimateRowHeight(rows[index]),
    getItemKey: useCallback((index: number) => rows[index]?.line.id ?? index, [rows]),
    overscan: OVERSCAN,
    paddingStart: PADDING_START_PX,
    paddingEnd: PADDING_END_PX,
    initialOffset: live ? Number.MAX_SAFE_INTEGER : 0,
  });
}

/** The list over `rows`; `live` follows the newest line. */
export function useTranscriptList(
  rows: readonly TranscriptRow[],
  live: boolean,
): TranscriptListController {
  const scrollRef = useRef<HTMLDivElement>(null);
  const { following, followingRef, isFollowing, follow } = useFollowing(live);
  const virtualizer = useRowVirtualizer(rows, live, scrollRef);
  return {
    scrollRef,
    sizerRef: useFollowObserver(scrollRef, isFollowing),
    virtualizer,
    following,
    onScroll: () => {
      const element = scrollRef.current;
      if (!live || element === null) return;
      const distance = element.scrollHeight - element.scrollTop - element.clientHeight;
      const atEnd = distance < FOLLOW_SLACK_PX;
      if (atEnd !== followingRef.current) follow(atEnd);
    },
    reveal: (row) => {
      if (live) follow(false);
      virtualizer.scrollToIndex(row, { align: 'center' });
    },
    jumpToNow: () => {
      follow(true);
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end' });
      pinToEnd(scrollRef.current);
    },
  };
}
