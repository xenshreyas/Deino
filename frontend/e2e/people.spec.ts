import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/** Saves a screenshot of the page in the light and the dark theme. */
async function bothThemes(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
}

test('People lists everyone by last met and opens a person page', async ({ page }) => {
  await page.goto('/people');
  const list = page.getByRole('list', { name: 'People' });
  await expect(list.getByRole('link').first()).toBeVisible();
  await bothThemes(page, 'people');

  await page.getByRole('searchbox', { name: 'Search people' }).fill('maya');
  await list.getByRole('link', { name: /Maya Chen/ }).click();

  await expect(page.getByRole('heading', { name: 'Maya Chen' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'About' })).toContainText('webhook queue');
  await expect(page.getByRole('group', { name: 'Ask about Maya Chen' })).toContainText('WIP');
  await expect(
    page.getByRole('region', { name: 'Meetings' }).getByRole('link').first(),
  ).toBeVisible();
  await bothThemes(page, 'person');
});

test('a duplicate person merges into the one the page shows', async ({ page }) => {
  await page.goto('/people');
  await page.getByRole('button', { name: 'Add person' }).click();
  const form = page.getByRole('dialog', { name: 'New person' });
  await form.getByLabel('Name').fill('M. Chen');
  await form.getByLabel('Email').fill('maya.chen@gmail.com');
  await form.getByRole('button', { name: 'Create' }).click();
  await expect(page.getByRole('heading', { name: 'M. Chen' })).toBeVisible();

  // In-app navigation: a reload would reset the mock's session.
  const sidebar = page.getByRole('complementary', { name: 'Sidebar' });
  await sidebar.getByRole('link', { name: 'People' }).click();
  await page
    .getByRole('list', { name: 'People' })
    .getByRole('link', { name: /Maya Chen/ })
    .click();
  await page.getByRole('button', { name: 'More actions' }).click();
  await page.getByRole('menuitem', { name: 'Merge duplicate…' }).click();
  const merge = page.getByRole('dialog', { name: 'Merge a duplicate into Maya Chen' });
  await merge.getByRole('option', { name: /M\. Chen/ }).click();
  await merge.getByRole('button', { name: 'Merge' }).click();
  await expect(page.getByText('Merged into Maya Chen')).toBeVisible();

  await sidebar.getByRole('link', { name: 'People' }).click();
  await expect(page.getByRole('list', { name: 'People' })).toBeVisible();
  await expect(page.getByRole('link', { name: /M\. Chen/ })).toHaveCount(0);
});

test('a company page lists its people', async ({ page }) => {
  await page.goto('/companies');
  await page.getByRole('link', { name: /Brightkite Logistics/ }).click();

  await expect(page.getByRole('heading', { name: 'Brightkite Logistics' })).toBeVisible();
  const people = page.getByRole('region', { name: 'People' });
  await expect(people.getByRole('link', { name: /Marcus Webb/ })).toBeVisible();
  await bothThemes(page, 'company');
});

test('the meeting header lists attendees with their cards', async ({ page }) => {
  const discovery = fixtureMeeting('meeting-fixture-sales-discovery');
  await page.goto(`/meetings/${discovery.id}`);

  await page.getByRole('button', { name: /4 people/ }).click();
  const marcus = page
    .getByRole('list', { name: 'Attendees' })
    .getByRole('link', { name: /Marcus Webb/ });
  await marcus.hover();

  await expect(page.getByText('Recent meetings')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Open page' })).toBeVisible();
  await bothThemes(page, 'attendees');
});
