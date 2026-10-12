/**
 * TanStack Query over comments. The list names the comment tables, the
 * segments (a re-transcription detaches threads) and `meetings` (the
 * trash), so `DbChanged` keeps it fresh and writes need no invalidation.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { Table } from '@/lib/ipc/generated/shared';
import { IpcError } from '@/lib/ipc/result';

import { commentsApi } from './api';
import { commentsCopy } from './copy';
import type { CommentId, CommentThreadId, MeetingId, NewThread } from './types';

const COMMENT_TABLES = [
  'comment_threads',
  'comments',
  'transcript_segments',
  'meetings',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `commentKeys.all`. */
export const commentKeys = {
  all: ['comments'] as const,
  meeting: (meetingId: MeetingId) => [...commentKeys.all, meetingId] as const,
};

/** A meeting's threads. */
export const threadsQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: commentKeys.meeting(meetingId),
    queryFn: () => commentsApi.list(meetingId),
    meta: { tables: COMMENT_TABLES },
  });

/** A meeting's threads, oldest first. */
export function useThreads(meetingId: MeetingId) {
  return useQuery(threadsQuery(meetingId));
}

/** The error's message for a toast. */
function toastError(error: unknown): void {
  toast.error(error instanceof IpcError ? error.message : commentsCopy.failed);
}

/** Starts a thread. */
export function useStartThread() {
  return useMutation({
    mutationFn: (thread: NewThread) => commentsApi.start(thread),
    onError: toastError,
  });
}

/** Replies to a thread. */
export function useReply() {
  return useMutation({
    mutationFn: ({ threadId, body }: { threadId: CommentThreadId; body: string }) =>
      commentsApi.reply(threadId, body),
    onError: toastError,
  });
}

/** Resolves or reopens a thread. */
export function useSetResolved() {
  return useMutation({
    mutationFn: ({ threadId, resolved }: { threadId: CommentThreadId; resolved: boolean }) =>
      commentsApi.setResolved(threadId, resolved),
    onError: toastError,
  });
}

/** Deletes a comment. */
export function useDeleteComment() {
  return useMutation({
    mutationFn: (id: CommentId) => commentsApi.delete(id),
    onError: toastError,
  });
}
