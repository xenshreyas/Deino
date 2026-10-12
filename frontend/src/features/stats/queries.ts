/**
 * TanStack Query hooks for stats.
 *
 * Every query names the tables it is computed from, so `DbChanged` refreshes
 * the page after a recording, a merge, a trash or an LLM call. A range switch
 * keeps the previous numbers on screen until the new ones arrive.
 */
import {
  keepPreviousData,
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { statsApi } from './api';
import type { StatsRange } from './types';

/** What the numbers are computed from. */
const STATS_TABLES = [
  'meetings',
  'meeting_attendees',
  'people',
  'llm_calls',
] as const satisfies readonly Table[];
/** What the heatmap is computed from. */
const HEATMAP_TABLES = ['meetings'] as const satisfies readonly Table[];
/** Where the recap is kept. */
const RECAP_TABLES = ['year_recaps'] as const satisfies readonly Table[];

/** Query keys; every key starts with `statsKeys.all`. */
export const statsKeys = {
  all: ['stats'] as const,
  overview: (range: StatsRange) => [...statsKeys.all, 'overview', range] as const,
  heatmap: () => [...statsKeys.all, 'heatmap'] as const,
  recap: () => [...statsKeys.all, 'recap'] as const,
};

/** The numbers of `range`. Also usable from route loaders. */
export const statsOverviewQuery = (range: StatsRange) =>
  queryOptions({
    queryKey: statsKeys.overview(range),
    queryFn: () => statsApi.overview(range),
    meta: { tables: STATS_TABLES },
  });

/** The numbers of `range`, holding the last range's while loading. */
export function useStatsOverview(range: StatsRange) {
  return useQuery({ ...statsOverviewQuery(range), placeholderData: keepPreviousData });
}

/** Meetings per day over the last year. */
export const heatmapQuery = () =>
  queryOptions({
    queryKey: statsKeys.heatmap(),
    queryFn: statsApi.heatmap,
    meta: { tables: HEATMAP_TABLES },
  });

/** Meetings per day over the last year. */
export function useHeatmap() {
  return useQuery(heatmapQuery());
}

/** This year's recap and whether it can be generated. */
export function useRecapStatus() {
  return useQuery({
    queryKey: statsKeys.recap(),
    queryFn: statsApi.recapStatus,
    meta: { tables: RECAP_TABLES },
  });
}

/** Generates this year's recap, then refreshes its status. */
export function useGenerateRecap() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: statsApi.generateRecap,
    onSettled: () => queryClient.invalidateQueries({ queryKey: statsKeys.recap() }),
  });
}
