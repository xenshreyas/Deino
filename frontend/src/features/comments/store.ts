/**
 * What the comment popovers show (Zustand): the comment being written on a
 * selection, or the thread opened from its highlight. One at a time.
 */
import { create } from 'zustand';

import type { CommentAnchor, CommentThreadId, MeetingId, ScreenRect } from './types';

/** A comment being written on a span. */
export interface CommentDraft {
  meetingId: MeetingId;
  anchor: CommentAnchor;
  /** The selection on screen, where the composer opens. */
  rect: ScreenRect;
}

/** A thread opened from its highlight. */
export interface OpenThread {
  threadId: CommentThreadId;
  rect: ScreenRect;
}

interface CommentsUiState {
  draft: CommentDraft | null;
  open: OpenThread | null;
}

/** The popovers' state. */
export const useCommentsUi = create<CommentsUiState>()(() => ({ draft: null, open: null }));

/** Opens the composer on a span. */
export function startComment(draft: CommentDraft): void {
  useCommentsUi.setState({ draft, open: null });
}

/** Opens a thread by its highlight. */
export function openThread(open: OpenThread): void {
  useCommentsUi.setState({ open, draft: null });
}

/** Closes whichever popover is open. */
export function closeCommentPopover(): void {
  useCommentsUi.setState({ draft: null, open: null });
}

/** `rect` as a plain value (a `DOMRect` does not survive a state copy well). */
export function screenRect(rect: DOMRect): ScreenRect {
  return { left: rect.left, top: rect.top, width: rect.width, height: rect.height };
}
