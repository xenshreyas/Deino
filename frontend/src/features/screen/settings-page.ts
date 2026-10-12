/** Settings → Recording → Screen capture: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { ScreenCaptureSettings } from './components/screen-settings';

export const settingsSections = [
  { id: 'screen', page: 'recording', order: 60, Component: ScreenCaptureSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    screen: true;
  }
}
