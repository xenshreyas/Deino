import { describe, expect, test, vi } from 'vitest';

import { settingsSectionSchema } from './search';
import { SETTINGS_SECTIONS } from './sections';

// The inherited settings panels pull in Tauri and contexts; only the registry is under test.
vi.mock('@/components/About', () => ({ About: () => null }));
vi.mock('@/components/BetaSettings', () => ({ BetaSettings: () => null }));
vi.mock('@/components/PreferenceSettings', () => ({ PreferenceSettings: () => null }));
vi.mock('@/components/RecordingSettings', () => ({ RecordingSettings: () => null }));
vi.mock('@/components/SummaryModelSettings', () => ({ SummaryModelSettings: () => null }));
vi.mock('@/components/TranscriptSettings', () => ({ TranscriptSettings: () => null }));
vi.mock('@/contexts/ConfigContext', () => ({ useConfig: () => ({}) }));

describe('settings sections', () => {
  test('every page in the URL schema has exactly one registry entry (dev builds)', () => {
    expect(SETTINGS_SECTIONS.map((section) => section.id).sort()).toEqual(
      [...settingsSectionSchema.options].sort(),
    );
  });
});
