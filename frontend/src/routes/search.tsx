import { createFileRoute } from '@tanstack/react-router';

import { SearchScreen } from '@/features/search';
import { searchPageSchema } from '@/features/search/search';

/** `/search?q=&tab=&filters=`: every result of a search, by tab (ORG-08). */
export const Route = createFileRoute('/search')({
  validateSearch: searchPageSchema,
  component: SearchRoute,
});

function SearchRoute() {
  const params = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <SearchScreen
      params={params}
      onParamsChange={(next) =>
        void navigate({ search: (previous) => ({ ...previous, ...next }), replace: true })
      }
    />
  );
}
