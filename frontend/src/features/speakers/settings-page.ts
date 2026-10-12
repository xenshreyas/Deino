/** Settings → Recording → Speakers and Settings → Advanced → Voices: this feature's settings sections (REG-02, UX-18, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { SpeakerSettings, VoiceSettings } from './components/speaker-settings';

export const settingsSections = [
  { id: 'speakers', page: 'recording', order: 70, Component: SpeakerSettings },
  { id: 'voices', page: 'advanced', order: 40, aliases: ['people'], Component: VoiceSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    speakers: true;
    voices: true;
    people: true;
  }
}
