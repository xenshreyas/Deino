/**
 * Mock of `@/lib/ipc/generated/comments`: threads in memory with the Rust
 * rules (a body and a span are required, a transcript thread names a line,
 * a thread goes with its last comment).
 */
import type * as Generated from '@/lib/ipc/generated/comments';
import type { Table } from '@/lib/ipc/generated/shared';

import { failure, ok } from './command-result';
import { events } from './db';
import { latency } from './latency';

type Thread = Generated.CommentThread;

const COMMENT_TABLES: Table[] = ['comment_threads', 'comments'];

let seq = 5_000_000;
function changed() {
  seq += 1;
  void events.dbChanged.emit({ tables: COMMENT_TABLES, seq });
}

let threads: Thread[] = [];

const invalid = (message: string) => failure<never>({ code: 'invalid', message });
const missing = () =>
  failure<never>({ code: 'not_found', message: 'This comment no longer exists.' });

function comment(threadId: Generated.CommentThreadId, body: string): Generated.Comment {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID() as Generated.CommentId,
    threadId,
    body: body.trim(),
    createdAt: now,
    updatedAt: now,
  };
}

function validAnchor(anchor: Generated.CommentAnchor): boolean {
  const hasLine = anchor.segmentId !== null;
  const fits = anchor.target === 'transcript' ? hasLine : !hasLine;
  return fits && anchor.end > anchor.start && anchor.quote !== '';
}

function update(
  threadId: Generated.CommentThreadId,
  change: (thread: Thread) => Thread | null,
): boolean {
  const thread = threads.find(({ id }) => id === threadId);
  if (thread === undefined) return false;
  const next = change(thread);
  threads = threads.flatMap((each) =>
    each.id !== threadId ? [each] : next === null ? [] : [next],
  );
  changed();
  return true;
}

function threadOf(id: Generated.CommentId): Thread | undefined {
  return threads.find((thread) => thread.comments.some((each) => each.id === id));
}

export const commands = {
  async list(meetingId) {
    await latency();
    return ok(threads.filter((thread) => thread.meetingId === meetingId));
  },
  async start(thread) {
    await latency();
    if (thread.body.trim() === '') return invalid('Write a comment first.');
    if (!validAnchor(thread.anchor)) return invalid('Select some text to comment on.');
    const id = crypto.randomUUID() as Generated.CommentThreadId;
    const created: Thread = {
      id,
      meetingId: thread.meetingId,
      anchor: thread.anchor,
      resolvedAt: null,
      createdAt: new Date().toISOString(),
      comments: [comment(id, thread.body)],
    };
    threads = [...threads, created];
    changed();
    return ok(created);
  },
  async reply(threadId, body) {
    await latency();
    if (body.trim() === '') return invalid('Write a comment first.');
    const added = comment(threadId, body);
    const found = update(threadId, (thread) => ({
      ...thread,
      comments: [...thread.comments, added],
    }));
    return found ? ok(added) : missing();
  },
  async edit(id, body) {
    await latency();
    const thread = threadOf(id);
    if (thread === undefined) return missing();
    update(thread.id, (each) => ({
      ...each,
      comments: each.comments.map((c) => (c.id === id ? { ...c, body: body.trim() } : c)),
    }));
    return ok(null);
  },
  async setResolved(threadId, resolved) {
    await latency();
    const resolvedAt = resolved ? new Date().toISOString() : null;
    return update(threadId, (thread) => ({ ...thread, resolvedAt })) ? ok(null) : missing();
  },
  async delete(id) {
    await latency();
    const thread = threadOf(id);
    if (thread === undefined) return missing();
    update(thread.id, (each) => {
      const comments = each.comments.filter((c) => c.id !== id);
      return comments.length === 0 ? null : { ...each, comments };
    });
    return ok(null);
  },
} satisfies typeof Generated.commands;
