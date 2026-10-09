import { createFileRoute } from '@tanstack/react-router';

import { SettingsScreen } from '@/features/settings';

/** `/settings`: app settings. */
export const Route = createFileRoute('/settings')({ component: SettingsScreen });
