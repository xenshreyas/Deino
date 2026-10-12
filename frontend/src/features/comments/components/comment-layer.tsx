/**
 * Makes a transcript or notes view commentable (TRN-06): wraps it, shows
 * the selection menu over selected words (Comment, plus what the page adds:
 * Clip and Copy link in the transcript), paints the open threads' spans,
 * and opens a thread when its highlighted words are clicked.
 */
import { type MouseEvent, type ReactNode, useRef } from 'react';

import { cn } from '@/ui/cn';
import { MessageCircle } from '@/ui/icons';

import { commentsCopy } from '../copy';
import { type TextSelection, useTextSelection } from '../hooks/use-text-selection';
import { notesSurface, transcriptSurface } from '../model/surfaces';
import { useThreads } from '../queries';
import { openThread, screenRect, startComment } from '../store';
import type { CommentSurface, CommentTarget, CommentThread, MeetingId } from '../types';
import { CommentPopovers } from './comment-popovers';
import { HighlightPainter, paintedRanges } from './highlight-painter';
import { type SelectionAction, SelectionMenu } from './selection-menu';

/** Props of {@link CommentLayer}. */
export interface CommentLayerProps {
  meetingId: MeetingId;
  target: CommentTarget;
  /** More actions on a selection (the transcript's Clip and Copy link). */
  actions?: ((range: Range) => SelectionAction[]) | undefined;
  className?: string | undefined;
  children: ReactNode;
}

/** The surface a target's view is. */
function surfaceFor(target: CommentTarget): (container: HTMLElement) => CommentSurface {
  return target === 'transcript'
    ? transcriptSurface
    : (container) => notesSurface(container, target);
}

/** Whether the point is inside one of `range`'s line boxes. */
function hits(range: Range, x: number, y: number): boolean {
  return Array.from(range.getClientRects()).some(
    (box) => x >= box.left && x <= box.right && y >= box.top && y <= box.bottom,
  );
}

/** The thread whose painted words are under the click, if any. */
export function threadAt(
  surface: CommentSurface,
  threads: readonly CommentThread[],
  point: Pick<MouseEvent, 'clientX' | 'clientY'>,
): { thread: CommentThread; range: Range } | null {
  if (window.getSelection()?.isCollapsed === false) return null;
  for (const thread of threads) {
    const [range] = paintedRanges(surface, [thread]);
    if (range !== undefined && hits(range, point.clientX, point.clientY)) return { thread, range };
  }
  return null;
}

/** The Comment action on a selection, when it is on the surface. */
function commentAction(
  meetingId: MeetingId,
  surface: CommentSurface,
  selection: TextSelection,
): SelectionAction | null {
  const anchor = surface.anchorOf(selection.range);
  if (anchor === null) return null;
  const { rect } = selection;
  const run = () => {
    startComment({ meetingId, anchor, rect });
  };
  return { id: 'comment', label: commentsCopy.comment, icon: MessageCircle, run };
}

/** Every action of the selection menu. */
function menuFor(props: CommentLayerProps, selection: TextSelection): SelectionAction[] {
  const surface = surfaceFor(props.target)(selection.root);
  const comment = commentAction(props.meetingId, surface, selection);
  const more = props.actions?.(selection.range) ?? [];
  return comment === null ? more : [comment, ...more];
}

/** The view, commentable. */
export function CommentLayer(props: CommentLayerProps) {
  const ref = useRef<HTMLDivElement>(null);
  const selection = useTextSelection(ref);
  const threads = useThreads(props.meetingId);
  const surfaceOf = surfaceFor(props.target);
  const onClick = (event: MouseEvent) => {
    const root = ref.current;
    const hit = root === null ? null : threadAt(surfaceOf(root), threads.data ?? [], event);
    if (hit === null) return;
    openThread({ threadId: hit.thread.id, rect: screenRect(hit.range.getBoundingClientRect()) });
  };
  return (
    <div ref={ref} className={cn('min-h-0', props.className)} onClick={onClick}>
      {props.children}
      <HighlightPainter
        key={threads.dataUpdatedAt}
        container={ref}
        surface={surfaceOf}
        target={props.target}
        threads={threads.data ?? []}
      />
      <CommentPopovers meetingId={props.meetingId} />
      {selection === null ? null : (
        <SelectionMenu rect={selection.rect} actions={menuFor(props, selection)} />
      )}
    </div>
  );
}
