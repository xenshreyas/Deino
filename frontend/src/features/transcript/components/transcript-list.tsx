/**
 * The virtual list of bubbles: only the rows near the viewport are in the
 * DOM, each measured as it renders, so 5,000 lines scroll like 50. The
 * scroll area is one tab stop; ↑ and ↓ (and Home and End) move between
 * lines, and a focused line answers its actions' keys (⌘C copies it).
 */
import { type KeyboardEvent, useCallback, useState } from 'react';

import { actionForKey, type TranscriptAction } from '../actions';
import { transcriptCopy } from '../copy';
import type { TranscriptListController } from '../hooks/use-transcript-list';
import type { SearchHit, SearchResult } from '../model/search-index';
import type { MeetingId, TranscriptRow } from '../types';
import type { MomentMark } from './moment-focus';
import { TranscriptBubble } from './transcript-bubble';

/** Props of {@link TranscriptList}. */
export interface TranscriptListProps {
  meetingId: MeetingId;
  rows: readonly TranscriptRow[];
  list: TranscriptListController;
  result: SearchResult;
  current: SearchHit | undefined;
  /** The line a deep link marked, if any. */
  mark: MomentMark | null;
  actions: readonly TranscriptAction[];
}

/** The row a navigation key moves to from `index`, if it is one. */
function targetRow(key: string, index: number, count: number): number | undefined {
  if (key === 'ArrowDown') return Math.min(count - 1, index + 1);
  if (key === 'ArrowUp') return Math.max(0, index - 1);
  if (key === 'Home') return 0;
  if (key === 'End') return count - 1;
  return undefined;
}

/** Frames to wait for a far row to render after a scroll (Home, End). */
const FOCUS_ATTEMPTS = 8;

/** Focuses the bubble of `row` once the virtualiser has rendered it. */
function focusRow(container: HTMLElement | null, row: number, attempts = FOCUS_ATTEMPTS): void {
  requestAnimationFrame(() => {
    const bubble = container?.querySelector<HTMLElement>(`[data-bubble="${String(row)}"]`);
    if (bubble) bubble.focus();
    else if (attempts > 1) focusRow(container, row, attempts - 1);
  });
}

/** Keyboard moves and action keys, with the focused row. */
function useRowKeys({ rows, list, actions, meetingId }: TranscriptListProps) {
  const [focused, setFocused] = useState(-1);
  const { virtualizer, scrollRef } = list;
  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>, index: number) => {
      const row = rows[index];
      const action = actionForKey(actions, event);
      if (
        row !== undefined &&
        action !== undefined &&
        window.getSelection()?.isCollapsed !== false
      ) {
        event.preventDefault();
        void action.run({ meetingId, line: row.line, speaker: row.speaker });
        return;
      }
      const target = targetRow(event.key, index, rows.length);
      if (target === undefined) return;
      event.preventDefault();
      virtualizer.scrollToIndex(target, { align: 'auto' });
      focusRow(scrollRef.current, target);
    },
    [rows, virtualizer, scrollRef, actions, meetingId],
  );
  return { focused, onFocusRow: setFocused, onKeyDown };
}

/** The rows near the viewport, each placed at its measured offset. */
function VirtualRows(props: TranscriptListProps) {
  const { rows, list, result, current } = props;
  const keys = useRowKeys(props);
  return list.virtualizer.getVirtualItems().map((item) => {
    const row = rows[item.index];
    if (row === undefined) return null;
    return (
      <div
        key={item.key}
        ref={list.virtualizer.measureElement}
        data-index={item.index}
        className="absolute inset-x-0 top-0"
        style={{ transform: `translateY(${String(item.start)}px)` }}
      >
        <TranscriptBubble
          meetingId={props.meetingId}
          row={row}
          index={item.index}
          hits={result.byRow.get(item.index)}
          current={current?.row === item.index ? current.ordinal : undefined}
          marked={props.mark?.lineId === row.line.id}
          flashing={props.mark?.lineId === row.line.id && props.mark.flashing}
          focused={keys.focused === item.index}
          actions={props.actions}
          onFocusRow={keys.onFocusRow}
          onKeyDown={keys.onKeyDown}
        />
      </div>
    );
  });
}

/** ↑ or ↓ on the scroll area itself enters the list at the first visible line. */
function enterList(event: KeyboardEvent<HTMLDivElement>, list: TranscriptListController): void {
  if (event.target !== event.currentTarget) return;
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
  const first = list.virtualizer.getVirtualItems()[0];
  if (first === undefined) return;
  event.preventDefault();
  focusRow(list.scrollRef.current, first.index);
}

/** The list. */
export function TranscriptList(props: TranscriptListProps) {
  const { list } = props;
  return (
    <div
      ref={list.scrollRef}
      onScroll={list.onScroll}
      tabIndex={0}
      role="region"
      aria-label={transcriptCopy.label}
      onKeyDown={(event) => {
        enterList(event, list);
      }}
      className="min-h-0 flex-1 overflow-y-auto overscroll-contain rounded-md px-1 [scrollbar-gutter:stable] focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus-ring"
    >
      <div
        ref={list.sizerRef}
        role="list"
        className="relative w-full"
        style={{ height: list.virtualizer.getTotalSize() }}
      >
        <VirtualRows {...props} />
      </div>
    </div>
  );
}
