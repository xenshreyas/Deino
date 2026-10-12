import { createFileRoute } from '@tanstack/react-router';

import { heatmapQuery, StatsScreen, statsOverviewQuery } from '@/features/stats';

/** `/stats`: meetings and LLM usage in numbers, and the year in review (ORG-09). */
export const Route = createFileRoute('/stats')({
  loader: async ({ context }) => {
    await Promise.all([
      context.queryClient.query(statsOverviewQuery('last_12_months')).catch(() => undefined),
      context.queryClient.query(heatmapQuery()).catch(() => undefined),
    ]);
  },
  component: StatsScreen,
});
