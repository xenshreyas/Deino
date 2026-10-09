import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/automations`: Automations, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/automations')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="automations" />;
}
