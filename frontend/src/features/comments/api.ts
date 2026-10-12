/** Typed calls to the `comments` plugin (TRN-06). */
import { commands } from '@/lib/ipc/generated/comments';
import { unwrap } from '@/lib/ipc/result';

import type {
  Comment,
  CommentId,
  CommentThread,
  CommentThreadId,
  MeetingId,
  NewThread,
} from './types';

export const commentsApi = {
  /** The meeting's threads, oldest first. */
  list: (meetingId: MeetingId): Promise<CommentThread[]> => unwrap(commands.list(meetingId)),
  /** Starts a thread on a span with its first comment. */
  start: (thread: NewThread): Promise<CommentThread> => unwrap(commands.start(thread)),
  /** Adds a reply. */
  reply: (threadId: CommentThreadId, body: string): Promise<Comment> =>
    unwrap(commands.reply(threadId, body)),
  /** Replaces a comment's text. */
  edit: (id: CommentId, body: string): Promise<null> => unwrap(commands.edit(id, body)),
  /** Resolves or reopens a thread. */
  setResolved: (threadId: CommentThreadId, resolved: boolean): Promise<null> =>
    unwrap(commands.setResolved(threadId, resolved)),
  /** Deletes a comment (its thread goes with the last one). */
  delete: (id: CommentId): Promise<null> => unwrap(commands.delete(id)),
};
