/**
 * TanStack Query over the player's files. The query names `meetings`, whose
 * `audio_*` and `video_path` columns the `finalize_audio` stage writes, so
 * the dock appears once a recording is finalized.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { playerApi } from './api';
import type { MediaSource, MeetingId } from './types';

const MEDIA_TABLES = ['meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `playerKeys.all`. */
export const playerKeys = {
  all: ['player'] as const,
  media: (meetingId: MeetingId) => [...playerKeys.all, 'media', meetingId] as const,
};

/** A meeting's playable files. */
export const mediaQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: playerKeys.media(meetingId),
    queryFn: () => playerApi.media(meetingId),
    meta: { tables: MEDIA_TABLES },
  });

/** A meeting's playable files. */
export function useMedia(meetingId: MeetingId) {
  return useQuery(mediaQuery(meetingId));
}

/** Saves a copy of one of the meeting's files through the save dialog. */
export function useDownload(meetingId: MeetingId) {
  return useMutation({
    mutationFn: (source: MediaSource) => playerApi.download(meetingId, source),
  });
}
