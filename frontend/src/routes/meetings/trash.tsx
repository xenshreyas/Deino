import { createFileRoute } from '@tanstack/react-router';

import { TrashScreen } from '@/features/meetings-list';

/** `/meetings/trash`: meetings deleted in the last 30 days (ORG-07). */
export const Route = createFileRoute('/meetings/trash')({ component: TrashScreen });
