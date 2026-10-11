/**
 * TanStack Query hooks for the enhanced notes.
 *
 * A version's view carries its regenerate reasons, which change with the
 * calendar link, the attendees, the transcript and the user's notes, so it names those tables
 * too; `DbChanged` keeps everything fresh. Writing notes is one mutation
 * per meeting (`mutationKey`), so the header's Enhance button and the Notes
 * tab both see it running.
 */
import {
  queryOptions,
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import type { MeetingId } from '@/features/meetings';
import { usePipelineStatus } from '@/features/pipeline';
import type { Table } from '@/lib/ipc/generated/shared';

import { enhanceApi } from './api';
import type { EnhancedNotesView, SegmentId } from './types';

/** The pipeline stage that writes notes after a meeting. */
const ENHANCE_STAGE = 'enhance_notes';

/** What a version's view reads (its regenerate reasons included). */
const VIEW_TABLES = [
  'enhanced_notes',
  'meetings',
  'meeting_notes',
  'meeting_attendees',
  'transcript_segments',
  'templates',
] as const satisfies readonly Table[];
/** What the version list reads. */
const VERSION_TABLES = ['enhanced_notes', 'templates'] as const satisfies readonly Table[];
/** What cited excerpts read. */
const CITED_TABLES = [
  'transcript_segments',
  'speakers',
  'people',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `enhanceKeys.all`. */
export const enhanceKeys = {
  all: ['enhanced'] as const,
  view: (id: MeetingId, version: number | null) =>
    [...enhanceKeys.all, 'view', id, version ?? 'newest'] as const,
  versions: (id: MeetingId) => [...enhanceKeys.all, 'versions', id] as const,
  cited: (id: MeetingId, segments: readonly string[]) =>
    [...enhanceKeys.all, 'cited', id, ...segments] as const,
  run: (id: MeetingId) => [...enhanceKeys.all, 'run', id] as const,
};

/** A version of the meeting's notes; the newest for `null`. */
export const enhancedNotesQuery = (id: MeetingId, version: number | null) =>
  queryOptions({
    queryKey: enhanceKeys.view(id, version),
    queryFn: () => enhanceApi.get(id, version),
    meta: { tables: VIEW_TABLES },
  });

/** A version of the meeting's notes; the newest for `null`. */
export function useEnhancedNotes(id: MeetingId, version: number | null = null) {
  return useQuery(enhancedNotesQuery(id, version));
}

/** Every version, newest first. */
export function useNoteVersions(id: MeetingId) {
  return useQuery({
    queryKey: enhanceKeys.versions(id),
    queryFn: () => enhanceApi.versions(id),
    meta: { tables: VERSION_TABLES },
  });
}

/** The excerpts behind `segments`, fetched once the popover opens. */
export function useCitedSegments(id: MeetingId, segments: readonly SegmentId[], enabled: boolean) {
  return useQuery({
    queryKey: enhanceKeys.cited(id, segments),
    queryFn: () => enhanceApi.cited(id, [...segments]),
    meta: { tables: CITED_TABLES },
    enabled,
  });
}

/** Writes new notes; the argument says whether the user agreed to replace edits. */
export function useEnhance(id: MeetingId) {
  const client = useQueryClient();
  return useMutation({
    mutationKey: enhanceKeys.run(id),
    mutationFn: (replaceEdits: boolean) => enhanceApi.enhance(id, replaceEdits),
    onSuccess: (view) => {
      client.setQueryData<EnhancedNotesView | null>(enhanceKeys.view(id, null), view);
    },
  });
}

/** Restores an older version as the newest. */
export function useRestoreVersion(id: MeetingId) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (version: number) => enhanceApi.restore(id, version),
    onSuccess: (view) => {
      client.setQueryData<EnhancedNotesView | null>(enhanceKeys.view(id, null), view);
    },
  });
}

/** True while notes are being written: from this window, or by the pipeline. */
export function useIsEnhancing(id: MeetingId): boolean {
  const mutating = useIsMutating({ mutationKey: enhanceKeys.run(id) });
  const pipeline = usePipelineStatus(id);
  const stage = pipeline.data?.find((candidate) => candidate.stage === ENHANCE_STAGE);
  return mutating > 0 || stage?.status === 'running';
}
