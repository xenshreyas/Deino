import { describe, expect, test, vi } from 'vitest';

import { settingsSectionSchema } from './search';
import { SETTINGS_GROUPS, settingsSections } from './sections';

// The inherited settings panels pull in Tauri and contexts; only the registry is under test.
vi.mock('@/components/About', () => ({ About: () => null }));
vi.mock('@/components/PreferenceSettings', () => ({ PreferenceSettings: () => null }));
vi.mock('@/components/RecordingSettings', () => ({ RecordingSettings: () => null }));
vi.mock('@/components/SummaryModelSettings', () => ({ SummaryModelSettings: () => null }));
vi.mock('./components/engine-settings', () => ({ EngineSettings: () => null }));
vi.mock('@/contexts/ConfigContext', () => ({ useConfig: () => ({}) }));

/** The pages and their nav order before pages registered themselves (REG-02). */
const PAGES_BEFORE_REG_02 = [
  ['app', 'general', 'appearance', 'shortcuts', 'setup'],
  [
    'capture',
    'recording',
    'detection',
    'transcription',
    'vocabulary',
    'watched-folders',
    'people',
    'speaker-tags',
    'consent',
    'screen',
  ],
  ['ai', 'ai', 'models', 'templates', 'profile', 'search'],
  ['workspace', 'calendar', 'integrations'],
  ['privacy', 'security', 'network'],
  ['data', 'import', 'export'],
  ['more', 'about', 'developer'],
] as const;

describe('settings sections', () => {
  test('every feature page registers once, with its copy', () => {
    const ids = settingsSections().map((page) => page.id);

    expect(new Set(ids).size).toBe(ids.length);
    for (const page of settingsSections()) {
      expect(page.label.length).toBeGreaterThan(0);
      expect(page.description.length).toBeGreaterThan(0);
    }
  });

  test('pages keep their groups and order', () => {
    const known = new Set<string>(PAGES_BEFORE_REG_02.flatMap(([, ...pages]) => pages));
    const placed = settingsSections()
      .filter((page) => known.has(page.id))
      .map((page) => [page.group, page.id]);

    expect(placed).toEqual(
      PAGES_BEFORE_REG_02.flatMap(([group, ...pages]) => pages.map((id) => [group, id])),
    );
  });

  test('groups appear in nav order', () => {
    const groups = settingsSections().map((page) => SETTINGS_GROUPS.indexOf(page.group));

    expect(groups).toEqual([...groups].sort((a, b) => a - b));
    expect(settingsSections()[0]?.id).toBe('general');
  });

  test('the URL accepts registered pages only', () => {
    expect(settingsSectionSchema.safeParse('screen').success).toBe(true);
    expect(settingsSectionSchema.safeParse('nope').success).toBe(false);
    expect(settingsSectionSchema.safeParse(3).success).toBe(false);
  });
});
