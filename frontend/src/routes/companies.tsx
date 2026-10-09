import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/companies`: Companies, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/companies')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="companies" />;
}
