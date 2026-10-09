import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/people`: People, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/people')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="people" />;
}
