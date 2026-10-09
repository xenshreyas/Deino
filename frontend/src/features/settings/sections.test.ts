import { describe, expect, test, vi } from 'vitest';

import { settingsCopy } from './copy';
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

  test('Appearance sits in the App group, right after General', () => {
    expect(SETTINGS_SECTIONS.slice(0, 2).map(({ id, group }) => [id, group])).toEqual([
      ['general', 'app'],
      ['appearance', 'app'],
    ]);
    expect(settingsCopy.sections.appearance.label).toBe('Appearance');
  });
});
