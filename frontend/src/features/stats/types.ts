/**
 * The stats feature's types: the generated IPC types, re-exported so the
 * rest of the app never imports from `lib/ipc/generated` directly.
 */
export type {
  AudienceSplit,
  Heatmap,
  HeatmapDay,
  LlmUsage,
  ModelUsage,
  RecapStatus,
  StatsOverview,
  StatsRange,
  TopPerson,
  YearRecap,
} from '@/lib/ipc/generated/stats';

import type { StatsRange } from '@/lib/ipc/generated/stats';

/** The ranges the page offers, in the order the control shows them. */
export const STATS_RANGES = [
  'last_30_days',
  'last_90_days',
  'last_12_months',
  'all_time',
] as const satisfies readonly StatsRange[];
