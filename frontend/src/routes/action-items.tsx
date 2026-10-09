import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/action-items`: Action Items, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/action-items')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="action-items" />;
}
