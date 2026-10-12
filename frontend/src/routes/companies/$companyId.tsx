import { createFileRoute } from '@tanstack/react-router';

import { asCompanyId, CompanyScreen, companyPageQuery } from '@/features/entities';

/** `/companies/$companyId`: one company's page (ORG-04). */
export const Route = createFileRoute('/companies/$companyId')({
  // Warm the page's query; the page shows a missing company itself.
  loader: async ({ context, params }) => {
    await context.queryClient
      .query(companyPageQuery(asCompanyId(params.companyId)))
      .catch(() => undefined);
  },
  component: CompanyRoute,
});

function CompanyRoute() {
  const { companyId } = Route.useParams();
  // Remount per company so the notes field starts from that company's notes.
  return <CompanyScreen key={companyId} companyId={asCompanyId(companyId)} />;
}
