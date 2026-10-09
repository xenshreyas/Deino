import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/folders`: Folders, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/folders')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="folders" />;
}
