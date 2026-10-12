/** Settings → Screen capture: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Monitor } from '@/ui/icons';

import { ScreenCaptureSettings } from './components/screen-settings';

export const settingsPages = [
  {
    id: 'screen',
    group: 'capture',
    order: 90,
    icon: Monitor,
    label: 'Screen capture',
    description: 'Snapshots and video of the meeting window, kept on this Mac.',
    Component: ScreenCaptureSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    screen: true;
  }
}
