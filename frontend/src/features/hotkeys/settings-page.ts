/** Settings → Shortcuts: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Keyboard } from '@/ui/icons';

import { HotkeySettings } from './components/hotkey-settings';

export const settingsPages = [
  {
    id: 'shortcuts',
    group: 'app',
    order: 30,
    icon: Keyboard,
    label: 'Shortcuts',
    description: 'Hotkeys that work in every app, and deino:// links.',
    Component: HotkeySettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    shortcuts: true;
  }
}
