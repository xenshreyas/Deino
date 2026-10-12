import { screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { Info, Settings } from '@/ui/icons';

import type { SettingsSectionId } from './search';
import type { SettingsPage, SettingsPageSection, SettingsRegistry } from './sections';
import { SettingsScreen } from './settings-screen';
import { renderInSettings } from './test-render';

/** Two pages, three sections, one alias: enough to see placement and anchors. */
const PAGES: SettingsPage[] = [
  { id: 'general', order: 10, icon: Settings, label: 'General', description: 'First page.' },
  { id: 'advanced', order: 20, icon: Info, label: 'Advanced', description: 'Second page.' },
];

/** A section that renders its id. */
function section(id: SettingsSectionId, page: SettingsSectionId, order: number) {
  return { id, page, order, aliases: [], Component: () => <h3>{`Section ${id}`}</h3> };
}

const REGISTRY: SettingsRegistry = {
  pages: PAGES,
  sections: [
    section('appearance', 'general', 10),
    section('search', 'advanced', 20),
    { ...section('voices', 'advanced', 30), aliases: ['people'] } satisfies SettingsPageSection,
  ],
};

vi.mock('./sections', async (importActual) => {
  const actual = await importActual<typeof import('./sections')>();
  return {
    ...actual,
    settingsPages: () => REGISTRY.pages,
    sectionsOf: (page: string) => REGISTRY.sections.filter((entry) => entry.page === page),
    resolveSettingsTarget: (id: string) => actual.resolveIn(REGISTRY, id),
  };
});

/** The screen at `/settings` with `section` open. */
function renderScreen(section: SettingsSectionId) {
  renderInSettings(() => <SettingsScreen section={section} />);
}

describe('SettingsScreen', () => {
  const scrollIntoView = vi.fn();

  beforeEach(() => {
    Element.prototype.scrollIntoView = scrollIntoView;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('lists the pages without group headings and opens one', async () => {
    renderScreen('general');

    const nav = await screen.findByRole('navigation', { name: 'Settings pages' });
    expect(
      within(nav)
        .getAllByRole('link')
        .map((link) => link.textContent),
    ).toEqual(['General', 'Advanced']);
    expect(within(nav).queryAllByRole('heading', { level: 2 })).toHaveLength(0);
    expect(screen.getByRole('heading', { name: 'General', level: 2 })).toBeTruthy();
    expect(screen.getByText('Section appearance')).toBeTruthy();
    expect(screen.queryByText('Section search')).toBeNull();
    expect(scrollIntoView).not.toHaveBeenCalled();
  });

  it('opens a section on its host page, scrolled to it', async () => {
    renderScreen('search');

    expect(await screen.findByRole('heading', { name: 'Advanced', level: 2 })).toBeTruthy();
    expect(screen.getByText('Section voices')).toBeTruthy();
    expect(scrollIntoView).toHaveBeenCalledOnce();
    expect(scrollIntoView.mock.contexts[0]).toBe(
      document.getElementById('settings-section-search'),
    );
  });

  it('opens a retired page id at the section that replaced it', async () => {
    renderScreen('people');

    expect(await screen.findByRole('heading', { name: 'Advanced', level: 2 })).toBeTruthy();
    expect(scrollIntoView.mock.contexts[0]).toBe(
      document.getElementById('settings-section-voices'),
    );
  });
});
