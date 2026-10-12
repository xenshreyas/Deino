/**
 * The transcript selection menu's Clip and Copy link (TRN-04): the selected
 * words timed by their own timestamps when the engine gave them (else by
 * their lines'), from the first selected word's start to the last one's end.
 */
import { clipSpan, clipsCopy, copySpanLink, type TimedSpan } from '@/features/clips';
import { bubbleOf, SEGMENT_ATTRIBUTE, type SelectionAction, textOffset } from '@/features/comments';
import { type Transcript, timedWords } from '@/features/transcript';
import { Link, Scissors } from '@/ui/icons';

import type { MeetingId } from '@/features/meetings';

type Segment = Transcript['segments'][number];

/** The time at character `offset` of `segment`: its word's start (or end), else the line's. */
export function timeAt(segment: Segment, offset: number, edge: 'start' | 'end'): number {
  const words = timedWords(segment.text, segment.words) ?? [];
  const word =
    edge === 'start'
      ? words.find(({ range }) => range.end > offset)
      : words.filter(({ range }) => range.start < offset).at(-1);
  if (word === undefined) return edge === 'start' ? segment.start : segment.end;
  return edge === 'start' ? word.start : word.end;
}

/** The line and offset of one end of a selection. */
function endpoint(
  container: HTMLElement,
  segments: ReadonlyMap<string, Segment>,
  node: Node,
  offset: number,
) {
  const bubble = bubbleOf(container, node);
  const segment = segments.get(bubble?.getAttribute(SEGMENT_ATTRIBUTE) ?? '');
  if (bubble === null || segment === undefined) return null;
  return { segment, offset: textOffset(bubble, node, offset) ?? 0 };
}

/** The selected words of `range`, timed; `null` outside saved lines. */
export function selectionSpan(
  container: HTMLElement,
  range: Range,
  segments: ReadonlyMap<string, Segment>,
): TimedSpan | null {
  const from = endpoint(container, segments, range.startContainer, range.startOffset);
  const to = endpoint(container, segments, range.endContainer, range.endOffset);
  if (from === null || to === null) return null;
  const start = timeAt(from.segment, from.offset, 'start');
  const end = timeAt(to.segment, to.offset, 'end');
  return end > start ? { start, end, text: range.toString() } : null;
}

/** Clip and Copy link for a selection, when it is timed. */
export function selectionActions(
  meetingId: MeetingId,
  span: TimedSpan | null,
  playable: boolean,
): SelectionAction[] {
  if (span === null) return [];
  const link: SelectionAction = {
    id: 'link',
    label: clipsCopy.copyLink,
    icon: Link,
    run: () => void copySpanLink(meetingId, span),
  };
  if (!playable) return [link];
  const clip: SelectionAction = {
    id: 'clip',
    label: clipsCopy.clip,
    icon: Scissors,
    run: () => {
      clipSpan(meetingId, span);
    },
  };
  return [clip, link];
}
