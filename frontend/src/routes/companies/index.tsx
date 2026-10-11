import { createFileRoute } from '@tanstack/react-router';

import { CompaniesScreen, companiesListQuery } from '@/features/entities';

/** `/companies`: every organisation behind the user's meetings (ORG-04). */
export const Route = createFileRoute('/companies/')({
  loader: async ({ context }) => {
    await context.queryClient.query(companiesListQuery()).catch(() => undefined);
  },
  component: CompaniesScreen,
});
