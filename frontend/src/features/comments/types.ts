/** Types of comments: the generated ones and where the UI shows them. */
export type {
  Comment,
  CommentAnchor,
  CommentId,
  CommentTarget,
  CommentThread,
  CommentThreadId,
  MeetingId,
  NewThread,
  SegmentId,
} from '@/lib/ipc/generated/comments';

import type { CommentAnchor, CommentThread } from '@/lib/ipc/generated/comments';

/** A rectangle on screen, where a popover opens. */
export interface ScreenRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * A surface comments live on (the transcript, My notes, the enhanced
 * notes): it turns a selected DOM range into an anchor, and an anchor back
 * into a DOM range for the highlight.
 */
export interface CommentSurface {
  /** The anchor of `range`, or `null` when it is not on this surface. */
  anchorOf: (range: Range) => CommentAnchor | null;
  /** The thread's span on screen, or `null` (not drawn now, or detached). */
  rangeOf: (thread: CommentThread) => Range | null;
  /** Whether this surface shows `thread`. */
  shows: (thread: CommentThread) => boolean;
}
