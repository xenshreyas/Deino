import { createFileRoute } from '@tanstack/react-router';

import { automationListQuery, AutomationsScreen } from '@/features/automations';
import { automationsSearchSchema } from '@/features/automations/search';

/** `/automations?automation=<id|new>`: every automation, one builder at a time. */
export const Route = createFileRoute('/automations')({
  validateSearch: automationsSearchSchema,
  // Warm the list; the page shows its own loading state.
  loader: async ({ context }) => {
    await context.queryClient.query(automationListQuery()).catch(() => undefined);
  },
  component: AutomationsRoute,
});

function AutomationsRoute() {
  const { automation } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <AutomationsScreen
      selected={automation}
      onSelect={(selection) => void navigate({ search: { automation: selection }, replace: true })}
    />
  );
}
