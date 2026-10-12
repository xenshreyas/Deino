/**
 * The stats feature (ORG-09): the Stats page over the `stats` plugin, with
 * the activity heatmap, the period's numbers, LLM usage and the year in
 * review. Other code imports from here only.
 */
export { statsApi } from './api';
export { StatsScreen } from './components/stats-screen';
export { statsCopy } from './copy';
export {
  heatmapQuery,
  statsKeys,
  statsOverviewQuery,
  useGenerateRecap,
  useHeatmap,
  useRecapStatus,
  useStatsOverview,
} from './queries';
export type { Heatmap, RecapStatus, StatsOverview, StatsRange, YearRecap } from './types';
