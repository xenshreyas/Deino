/**
 * TanStack Query over clips. Lists name `clips` and `meetings` (a title
 * change or a meeting moved to the trash refreshes them through
 * `DbChanged`); writes need no manual invalidation for the same reason.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { Table } from '@/lib/ipc/generated/shared';
import { IpcError } from '@/lib/ipc/result';

import { clipsApi } from './api';
import { clipsCopy } from './copy';
import type { ClipId, MeetingId, NewClip } from './types';

const CLIP_TABLES = ['clips', 'meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `clipKeys.all`. */
export const clipKeys = {
  all: ['clips'] as const,
  meeting: (meetingId: MeetingId) => [...clipKeys.all, 'meeting', meetingId] as const,
  everything: () => [...clipKeys.all, 'all'] as const,
};

/** A meeting's clips, in time order. */
export const meetingClipsQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: clipKeys.meeting(meetingId),
    queryFn: () => clipsApi.list(meetingId),
    meta: { tables: CLIP_TABLES },
  });

/** Every clip, newest first (⌘K). */
export const allClipsQuery = () =>
  queryOptions({
    queryKey: clipKeys.everything(),
    queryFn: clipsApi.listAll,
    meta: { tables: CLIP_TABLES },
  });

/** A meeting's clips. */
export function useMeetingClips(meetingId: MeetingId) {
  return useQuery(meetingClipsQuery(meetingId));
}

/** Every clip, for the palette. */
export function useAllClips() {
  return useQuery(allClipsQuery());
}

/** The error's message for a toast. */
function toastError(error: unknown): void {
  toast.error(error instanceof IpcError ? error.message : clipsCopy.failed);
}

/** Cuts a new clip. */
export function useCreateClip() {
  return useMutation({
    mutationFn: (clip: NewClip) => clipsApi.create(clip),
    onSuccess: () => toast.success(clipsCopy.created),
    onError: toastError,
  });
}

/** Renames a clip. */
export function useRenameClip() {
  return useMutation({
    mutationFn: ({ id, name }: { id: ClipId; name: string }) => clipsApi.rename(id, name),
    onError: toastError,
  });
}

/** Deletes a clip. */
export function useDeleteClip() {
  return useMutation({
    mutationFn: (id: ClipId) => clipsApi.delete(id),
    onSuccess: () => toast.success(clipsCopy.deleted),
    onError: toastError,
  });
}

/** Shows a clip in Finder. */
export function useRevealClip() {
  return useMutation({ mutationFn: (id: ClipId) => clipsApi.reveal(id), onError: toastError });
}
