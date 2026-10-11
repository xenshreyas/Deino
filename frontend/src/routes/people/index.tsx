import { createFileRoute } from '@tanstack/react-router';

import { PeopleScreen, peopleListQuery } from '@/features/entities';

/** `/people`: everyone the user meets (ORG-04). */
export const Route = createFileRoute('/people/')({
  loader: async ({ context }) => {
    await context.queryClient.query(peopleListQuery()).catch(() => undefined);
  },
  component: PeopleScreen,
});
