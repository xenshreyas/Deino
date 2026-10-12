/** Settings → General → Setup and Settings → Recording → Permissions: this feature's settings sections (REG-02, UX-10, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { PermissionsSettings } from './components/permissions-settings';
import { ResumeSetup } from './components/resume-setup';

export const settingsSections = [
  { id: 'setup', page: 'general', order: 40, Component: ResumeSetup },
  { id: 'permissions', page: 'recording', order: 10, Component: PermissionsSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    setup: true;
    permissions: true;
  }
}
