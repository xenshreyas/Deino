/** Settings → Advanced → Shortcuts and links: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { HotkeySettings } from './components/hotkey-settings';

export const settingsSections = [
  { id: 'shortcuts', page: 'advanced', order: 50, Component: HotkeySettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    shortcuts: true;
  }
}
