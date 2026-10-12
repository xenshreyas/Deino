/** Settings → Calendar: this feature's settings page (REG-02). */
import { BriefSettingsSection } from '@/features/briefs';
import type { SettingsPage } from '@/features/settings';
import { Calendar } from '@/ui/icons';

import { CalendarSettings } from './components/calendar-settings';

/** Sync and calendars (ORG-01), then pre-meeting briefs (NOT-08). */
function CalendarSection() {
  return (
    <div className="flex flex-col gap-8">
      <CalendarSettings />
      <BriefSettingsSection />
    </div>
  );
}

export const settingsPages = [
  {
    id: 'calendar',
    group: 'workspace',
    order: 10,
    icon: Calendar,
    label: 'Calendar',
    description: 'Your calendars from macOS Calendar, and which ones Deino shows.',
    Component: CalendarSection,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    calendar: true;
  }
}
