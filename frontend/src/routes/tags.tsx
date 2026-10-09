import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/tags`: Tags, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/tags')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="tags" />;
}
