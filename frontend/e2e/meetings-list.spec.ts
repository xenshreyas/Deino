/**
 * The Meetings page (ORG-07): filters, saved views as tabs, `F` filter
 * search, and selecting with Shift-click and ⌘A for bulk tagging and filing.
 */
import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';
import { shootBothThemes } from './themes';

const interview = fixtureMeeting('meeting-fixture-interview');
const standup = fixtureMeeting('meeting-fixture-standup');
const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
const support = fixtureMeeting('meeting-fixture-customer-support');

/** The row link of the meeting titled `title`. */
const row = (page: Page, title: string) => page.getByRole('link', { name: title });

/** The selection checkbox of the meeting titled `title`. */
const checkbox = (page: Page, title: string) =>
  page.getByRole('checkbox', { name: `Select ${title}`, exact: true });

const bulkBar = (page: Page) => page.getByRole('toolbar', { name: 'Selected meetings' });

test.beforeEach(async ({ page }) => {
  await page.goto('/meetings');
  await expect(row(page, standup.title)).toBeVisible();
});

test('the list groups meetings by day, newest first', async ({ page }) => {
  const days = page.getByRole('main').getByRole('heading', { level: 2 });
  await expect(days.first()).toHaveText('Thursday, October 8');
  await expect(days.nth(1)).toHaveText('Wednesday, October 7');
  const today = page.getByRole('region', { name: 'Thursday, October 8' });
  await expect(today.getByRole('link', { name: standup.title })).toBeVisible();
  await shootBothThemes(page, 'meetings-list');
});

test('a filter saved as a view becomes a tab to switch to', async ({ page }) => {
  await page.getByRole('button', { name: /^Filter/ }).click();
  await page.getByRole('option', { name: 'Hiring', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Remove filter: Hiring' })).toBeVisible();
  await expect(row(page, interview.title)).toBeVisible();
  await expect(row(page, standup.title)).toBeHidden();

  await page.getByRole('button', { name: 'Save view' }).click();
  await page.getByLabel('View name').fill('Hiring loop');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  const tabs = page.getByRole('tablist', { name: 'Views' });
  const saved = tabs.getByRole('tab', { name: 'Hiring loop' });
  await expect(saved).toHaveAttribute('aria-selected', 'true');
  await shootBothThemes(page, 'meetings-view');

  await tabs.getByRole('tab', { name: 'All meetings' }).click();
  await expect(row(page, standup.title)).toBeVisible();
  await saved.click();
  await expect(row(page, standup.title)).toBeHidden();
  await expect(row(page, interview.title)).toBeVisible();
});

test('F opens filter search, and Enter applies the first match', async ({ page }) => {
  await page.keyboard.press('f');
  const search = page.getByPlaceholder('Filter by…');
  await expect(search).toBeFocused();
  await search.fill('hiring');
  await expect(page.getByRole('option', { name: 'Hiring', exact: true })).toBeVisible();
  await shootBothThemes(page, 'meetings-filter-search');

  await page.keyboard.press('Enter');
  await expect(search).toBeHidden();
  await expect(row(page, interview.title)).toBeVisible();
  await expect(row(page, standup.title)).toBeHidden();
});

test('Shift-click selects a range and ⌘A selects every match', async ({ page }) => {
  await checkbox(page, standup.title).click();
  await checkbox(page, support.title).click({ modifiers: ['Shift'] });

  for (const title of [standup.title, oneOnOne.title, support.title]) {
    await expect(checkbox(page, title)).toHaveAttribute('aria-checked', 'true');
  }
  await expect(checkbox(page, interview.title)).toHaveAttribute('aria-checked', 'false');
  await expect(bulkBar(page).getByText('3 meetings selected')).toBeVisible();
  await shootBothThemes(page, 'meetings-selection');
  await page.setViewportSize({ width: 960, height: 720 });
  await expect(bulkBar(page).getByRole('button', { name: 'Delete' })).toBeInViewport();
  await shootBothThemes(page, 'meetings-selection-narrow');

  await page.keyboard.press('ControlOrMeta+a');
  const total = await page.getByRole('checkbox', { name: /^Select / }).count();
  await expect(bulkBar(page).getByText(`${String(total)} meetings selected`)).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(bulkBar(page)).toBeHidden();
});

test('the selection is tagged and added to a folder in one go', async ({ page }) => {
  await checkbox(page, standup.title).click();
  await checkbox(page, oneOnOne.title).click({ modifiers: ['Shift'] });

  await bulkBar(page).getByRole('button', { name: 'Tag' }).click();
  await page.getByRole('option', { name: 'Hiring', exact: true }).click();
  await bulkBar(page).getByRole('button', { name: 'Add to folder' }).click();
  await page.getByRole('option', { name: 'Customers', exact: true }).click();

  await expect(row(page, standup.title)).toContainText('Hiring');
  await expect(row(page, oneOnOne.title)).toContainText('Hiring');
  const sidebar = page.getByRole('complementary', { name: 'Sidebar' });
  await sidebar.getByRole('treeitem', { name: 'Customers' }).click();
  await expect(page.getByRole('link', { name: standup.title })).toBeVisible();
  await expect(page.getByRole('link', { name: oneOnOne.title })).toBeVisible();
});

test('several selected meetings export into a folder as one format', async ({ page }) => {
  await checkbox(page, standup.title).click();
  await checkbox(page, oneOnOne.title).click({ modifiers: ['Shift'] });

  await bulkBar(page).getByRole('button', { name: 'Export' }).click();
  await expect(page.getByRole('menuitem', { name: /PDF/ })).toBeVisible();
  await shootBothThemes(page, 'meetings-bulk-export');
  await page.getByRole('menuitem', { name: /Markdown/ }).click();

  await expect(page.getByText('Exported 2 meetings to Downloads')).toBeVisible();
});
