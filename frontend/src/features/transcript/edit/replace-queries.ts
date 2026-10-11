/**
 * Find and replace over IPC (TRN-02): the preview is a query (it names
 * `transcript_segments` and `meetings`, so an edit elsewhere recounts it),
 * the replacement a mutation. Rust's `DbChanged` refreshes the transcript.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { transcriptApi } from '../api';
import { transcriptKeys } from '../queries';
import type { MeetingId, ReplaceRequest } from '../types';

const PREVIEW_TABLES = ['transcript_segments', 'meetings'] as const satisfies readonly Table[];

/** Where find and replace looks, as the bar offers it. */
export type ReplaceReach = 'meeting' | 'all';

/** The request for finding `find` in `reach` from `meetingId`'s transcript. */
export function replaceRequest(
  meetingId: MeetingId,
  find: string,
  { reach, matchCase }: { reach: ReplaceReach; matchCase: boolean },
): ReplaceRequest {
  return {
    find,
    matchCase,
    scope: reach === 'all' ? { kind: 'all' } : { kind: 'meeting', meetingId },
  };
}

/** What replacing would change; asked for only while the confirmation is open. */
export const replacePreviewQuery = (request: ReplaceRequest) =>
  queryOptions({
    queryKey: [...transcriptKeys.all, 'replace-preview', request] as const,
    queryFn: () => transcriptApi.previewReplace(request),
    meta: { tables: PREVIEW_TABLES },
  });

/** The preview, fetched while `enabled`. */
export function useReplacePreview(request: ReplaceRequest, enabled: boolean) {
  return useQuery({ ...replacePreviewQuery(request), enabled });
}

/** Applies a find and replace. */
export function useReplace() {
  return useMutation({
    mutationFn: ({ request, replacement }: { request: ReplaceRequest; replacement: string }) =>
      transcriptApi.replace(request, replacement),
  });
}
