import { createFileRoute } from '@tanstack/react-router';

import { asPersonId, PersonScreen, personPageQuery } from '@/features/entities';

/** `/people/$personId`: one person's page (ORG-04). */
export const Route = createFileRoute('/people/$personId')({
  // Warm the page's query; the page shows a missing person itself.
  loader: async ({ context, params }) => {
    await context.queryClient
      .query(personPageQuery(asPersonId(params.personId)))
      .catch(() => undefined);
  },
  component: PersonRoute,
});

function PersonRoute() {
  const { personId } = Route.useParams();
  // Remount per person so the notes field starts from that person's notes.
  return <PersonScreen key={personId} personId={asPersonId(personId)} />;
}
