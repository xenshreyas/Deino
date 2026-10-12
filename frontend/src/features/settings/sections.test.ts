import { describe, expect, test, vi } from 'vitest';

import { Settings } from '@/ui/icons';

import { type SettingsSectionId, settingsSectionSchema } from './search';
import {
  checkRegistry,
  resolveSettingsTarget,
  sectionsOf,
  type SettingsPage,
  type SettingsPageSection,
  settingsPages,
} from './sections';

// The inherited settings panels pull in Tauri and contexts; only the registry is under test.
vi.mock('@/components/About', () => ({ About: () => null }));
vi.mock('@/components/RecordingsFolderSettings', () => ({ RecordingsFolderSettings: () => null }));
vi.mock('@/components/SummaryModelSettings', () => ({ OtherProviders: () => null }));
vi.mock('./components/engine-settings', () => ({ EngineSettings: () => null }));
vi.mock('@/contexts/ConfigContext', () => ({ useConfig: () => ({}) }));

/** The nine pages of the target IA (UX-19), in nav order. */
const PAGES = [
  ['general', 'General'],
  ['recording', 'Recording'],
  ['notes', 'Notes'],
  ['templates', 'Templates'],
  ['calendar', 'Calendar'],
  ['connections', 'Connections'],
  ['privacy', 'Privacy'],
  ['advanced', 'Advanced'],
  ['about', 'About'],
] as const;

/**
 * Every page id from before UX-19 (27 pages; Beta was deleted by UX-12), and where a link to it lands
 * now: the host page and the section it scrolls to (none for a page's top).
 */
const OLD_IDS = [
  ['general', 'general', undefined],
  ['appearance', 'general', 'appearance'],
  ['shortcuts', 'advanced', 'shortcuts'],
  ['setup', 'general', 'setup'],
  ['recording', 'recording', undefined],
  ['detection', 'recording', 'detection'],
  ['transcription', 'advanced', 'transcription'],
  ['vocabulary', 'notes', 'vocabulary'],
  ['watched-folders', 'connections', 'watched-folders'],
  ['people', 'advanced', 'voices'],
  ['speaker-tags', 'recording', 'speaker-tags'],
  ['consent', 'recording', 'consent'],
  ['screen', 'recording', 'screen'],
  ['ai', 'notes', 'ai'],
  ['models', 'notes', 'ai'],
  ['templates', 'templates', undefined],
  ['profile', 'notes', 'profile'],
  ['search', 'advanced', 'search'],
  ['calendar', 'calendar', undefined],
  ['integrations', 'connections', 'integrations'],
  ['security', 'privacy', 'security'],
  ['network', 'privacy', 'network'],
  ['import', 'connections', 'import'],
  ['export', 'connections', 'export'],
  ['about', 'about', undefined],
  ['developer', 'advanced', 'developer'],
] as const;

const PAGE: SettingsPage = {
  id: 'general',
  order: 10,
  icon: Settings,
  label: 'General',
  description: 'One page.',
};

/** A section for the registry checks. */
function section(
  id: SettingsSectionId,
  page: SettingsSectionId,
  aliases: SettingsSectionId[] = [],
): SettingsPageSection {
  return { id, page, order: 10, aliases, Component: () => null };
}

describe('settings registry', () => {
  test('nine pages, in order, with no groups', () => {
    expect(settingsPages().map((page) => [page.id, page.label])).toEqual(PAGES);
    for (const page of settingsPages()) expect(page.description.length).toBeGreaterThan(0);
  });

  test('every page has sections', () => {
    for (const [id] of PAGES) expect(sectionsOf(id).length).toBeGreaterThan(0);
  });

  test.each(OLD_IDS)('the old page id %s still lands on %s', (id, page, anchor) => {
    const target = resolveSettingsTarget(id);

    expect(target?.page.id).toBe(page);
    expect(target?.section).toBe(anchor);
    expect(settingsSectionSchema.safeParse(id).success).toBe(true);
  });

  test('the URL accepts registered ids only', () => {
    expect(settingsSectionSchema.safeParse('permissions').success).toBe(true);
    expect(settingsSectionSchema.safeParse('nope').success).toBe(false);
    expect(settingsSectionSchema.safeParse(3).success).toBe(false);
  });

  test('a section must sit on a registered page, and ids are unique', () => {
    const pages = [PAGE];

    expect(() => {
      checkRegistry({ pages, sections: [section('appearance', 'privacy')] });
    }).toThrow(/unknown page/);
    expect(() => {
      checkRegistry({
        pages,
        sections: [section('appearance', 'general'), section('appearance', 'general')],
      });
    }).toThrow(/twice/);
    expect(() => {
      checkRegistry({ pages, sections: [section('appearance', 'general', ['general'])] });
    }).toThrow(/twice/);
    expect(() => {
      checkRegistry({ pages, sections: [section('general', 'general')] });
    }).not.toThrow();
  });
});
