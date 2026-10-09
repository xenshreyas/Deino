import { createFileRoute } from '@tanstack/react-router';

import { RecordScreen } from '@/features/record/record-screen';

/** `/`: record a new meeting. */
export const Route = createFileRoute('/')({ component: RecordScreen });
