import { createFileRoute } from '@tanstack/react-router';

import { asInsightId, insightListQuery, InsightsScreen } from '@/features/insights';
import { insightsSearchSchema } from '@/features/insights/search';

/** `/insights?insight=<id>`: every insight, one table of answers at a time. */
export const Route = createFileRoute('/insights')({
  validateSearch: insightsSearchSchema,
  // Warm the list; the page shows its own loading state.
  loader: async ({ context }) => {
    await context.queryClient.query(insightListQuery()).catch(() => undefined);
  },
  component: InsightsRoute,
});

function InsightsRoute() {
  const { insight } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <InsightsScreen
      selected={insight === undefined ? undefined : asInsightId(insight)}
      onSelect={(id) => void navigate({ search: { insight: id }, replace: true })}
    />
  );
}
