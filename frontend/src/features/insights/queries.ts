/**
 * TanStack Query hooks for insights.
 *
 * Queries name the tables they read, so `DbChanged` refreshes them after any
 * write, including the `insights` pipeline stage's: a run on past meetings
 * shows its progress and answers as they land. Mutations also invalidate
 * the insight queries themselves, so a change shows at once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { insightsApi } from './api';
import type { InsightId, InsightInput, MeetingId } from './types';

/** Definitions with their runs on meetings (titles, and speakers by name). */
const RUN_TABLES = [
  'insight_defs',
  'insight_results',
  'meetings',
  'speakers',
  'people',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `insightKeys.all`. */
export const insightKeys = {
  all: ['insights'] as const,
  templates: () => [...insightKeys.all, 'templates'] as const,
  list: () => [...insightKeys.all, 'list'] as const,
  table: (id: InsightId) => [...insightKeys.all, 'table', id] as const,
  meeting: (meetingId: MeetingId) => [...insightKeys.all, 'meeting', meetingId] as const,
  candidates: (id: InsightId) => [...insightKeys.all, 'candidates', id] as const,
};

/** Every insight with its progress. Also usable from route loaders. */
export const insightListQuery = () =>
  queryOptions({
    queryKey: insightKeys.list(),
    queryFn: insightsApi.list,
    meta: { tables: RUN_TABLES },
  });

/** The template gallery (fixed; never refetched). */
export function useInsightTemplates() {
  return useQuery({
    queryKey: insightKeys.templates(),
    queryFn: insightsApi.templates,
    staleTime: Infinity,
  });
}

/** Every insight, oldest first. */
export function useInsights() {
  return useQuery(insightListQuery());
}

/** One insight's answers across meetings. */
export function useInsightTable(id: InsightId) {
  return useQuery({
    queryKey: insightKeys.table(id),
    queryFn: () => insightsApi.table(id),
    meta: { tables: RUN_TABLES },
  });
}

/** A meeting's Insights tab. */
export function useMeetingInsights(meetingId: MeetingId) {
  return useQuery({
    queryKey: insightKeys.meeting(meetingId),
    queryFn: () => insightsApi.forMeeting(meetingId),
    meta: { tables: RUN_TABLES },
  });
}

/** The past meetings an insight could run on. */
export function useRunCandidates(id: InsightId) {
  return useQuery({
    queryKey: insightKeys.candidates(id),
    queryFn: () => insightsApi.candidates(id),
    meta: { tables: [...RUN_TABLES, 'meeting_tags', 'meeting_folders', 'meeting_attendees'] },
  });
}

/** Refetches every insight query once a mutation settles. */
function useInvalidateInsights() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: insightKeys.all });
}

/** Creates an insight, or replaces one's definition when `id` is set. */
export function useSaveInsight() {
  const invalidate = useInvalidateInsights();
  return useMutation({
    mutationFn: ({ id, input }: { id: InsightId | null; input: InsightInput }) =>
      id === null ? insightsApi.create(input) : insightsApi.update(id, input),
    onSuccess: invalidate,
  });
}

/** Deletes an insight. */
export function useDeleteInsight() {
  const invalidate = useInvalidateInsights();
  return useMutation({ mutationFn: insightsApi.delete, onSuccess: invalidate });
}

/** Runs an insight on meetings in the background. */
export function useRunInsight() {
  const invalidate = useInvalidateInsights();
  return useMutation({
    mutationFn: ({ id, meetingIds }: { id: InsightId; meetingIds: MeetingId[] }) =>
      insightsApi.run(id, meetingIds),
    onSuccess: invalidate,
  });
}

/** Saves an insight's table as CSV. */
export function useExportInsightCsv() {
  return useMutation({ mutationFn: insightsApi.exportCsv });
}
