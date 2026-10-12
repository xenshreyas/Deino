/** Settings → Calendar: this feature's settings section (REG-02, UX-19). */
import { BriefSettingsSection } from '@/features/briefs';
import type { SettingsPageSection } from '@/features/settings';

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

export const settingsSections = [
  { id: 'calendar', page: 'calendar', order: 10, Component: CalendarSection },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    calendar: true;
  }
}
