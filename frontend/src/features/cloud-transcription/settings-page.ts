/**
 * Settings → Advanced → ElevenLabs (optional): this feature's settings
 * section (REG-02, UX-19), right after Transcription (order 20).
 */
import type { SettingsPageSection } from '@/features/settings';

import { ElevenLabsSettings } from './components/elevenlabs-settings';

export const settingsSections = [
  { id: 'elevenlabs', page: 'advanced', order: 25, Component: ElevenLabsSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    elevenlabs: true;
  }
}
