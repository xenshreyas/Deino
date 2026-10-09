import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/insights`: Insights, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/insights')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="insights" />;
}
