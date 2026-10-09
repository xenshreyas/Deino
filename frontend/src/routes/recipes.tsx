import { createFileRoute } from '@tanstack/react-router';

import { ComingSoonScreen } from '@/features/shell';

/** `/recipes`: Recipes, a placeholder until its feature card ships (see `nav.ts`). */
export const Route = createFileRoute('/recipes')({ component: Placeholder });

function Placeholder() {
  return <ComingSoonScreen navId="recipes" />;
}
