import { createFileRoute } from '@tanstack/react-router';

import { UnfiledScreen } from '@/features/folders';

/** `/folders/unfiled`: the "Unfiled" smart folder, meetings in no folder. */
export const Route = createFileRoute('/folders/unfiled')({ component: UnfiledScreen });
