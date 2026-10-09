import { createFileRoute } from '@tanstack/react-router';

import { SettingsScreen } from '@/features/settings';
import { settingsSearchSchema } from '@/features/settings/search';

/** `/settings?section=`: app settings, one page at a time. */
export const Route = createFileRoute('/settings')({
  validateSearch: settingsSearchSchema,
  component: SettingsRoute,
});

function SettingsRoute() {
  const { section = 'general' } = Route.useSearch();
  return <SettingsScreen section={section} />;
}
