import { createFileRoute } from '@tanstack/react-router';

import {
  ActionItemsScreen,
  asViewId,
  filterOptionsQuery,
  savedViewsQuery,
  type ViewChoice,
} from '@/features/action-items';
import { actionItemsSearchSchema } from '@/features/action-items/search';

/** `/action-items?view=&add=`: every action item, the user's own open ones first. */
export const Route = createFileRoute('/action-items')({
  validateSearch: actionItemsSearchSchema,
  // Warm the tabs and the filter bar; the page shows its own loading states.
  loader: async ({ context }) => {
    await Promise.all([
      context.queryClient.query(savedViewsQuery()).catch(() => undefined),
      context.queryClient.query(filterOptionsQuery()).catch(() => undefined),
    ]);
  },
  component: ActionItemsRoute,
});

function ActionItemsRoute() {
  const { view, add } = Route.useSearch();
  const navigate = Route.useNavigate();
  const choice: ViewChoice =
    view === undefined ? { kind: 'mine' } : { kind: 'saved', id: asViewId(view) };
  return (
    <ActionItemsScreen
      choice={choice}
      adding={add === true}
      onChoose={(next) =>
        void navigate({
          search: { view: next.kind === 'saved' ? next.id : undefined },
          replace: true,
        })
      }
    />
  );
}
