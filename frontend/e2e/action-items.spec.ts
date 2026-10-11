import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/** Screenshots a page in both themes. */
async function shoot(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the theme's colour transitions finish before the capture.
    await page.waitForTimeout(400);
    await page.screenshot({
      path: test.info().outputPath(`${name}-${colorScheme}.png`),
      fullPage: true,
    });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

/** The fixture items that are the user's and overdue whatever today is (due in early October 2026). */
const JORDAN_DOC = 'Send Jordan the doc on how on-call and remote work at Deino';
const RUNBOOK =
  'Write the incident communication rules (owners, hourly updates, single channel) into the incident runbook';

test.beforeEach(async ({ page }) => {
  await page.goto('/action-items');
  await expect(page.getByRole('heading', { name: 'Action items' })).toBeVisible();
});

test('the page lists my items by due date, and ticking one completes it', async ({ page }) => {
  await expect(page.getByRole('region', { name: 'Overdue' })).toBeVisible();
  await shoot(page, 'action-items');

  await page.getByRole('checkbox', { name: `Mark “${JORDAN_DOC}” done` }).click();

  await expect(page.getByRole('button', { name: JORDAN_DOC, exact: true })).toHaveCount(0);
});

test('F opens the filter menu, and Overdue narrows the list', async ({ page }) => {
  await page.keyboard.press('f');
  await page.getByRole('option', { name: 'Overdue' }).click();

  await expect(page.getByRole('button', { name: 'Remove filter: Overdue' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Overdue' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Later' })).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'No date' })).toHaveCount(0);
  await shoot(page, 'action-items-overdue');
});

test('a range of selected items is reassigned together', async ({ page }) => {
  await page.getByRole('button', { name: `Select “${JORDAN_DOC}”` }).click();
  await page.getByRole('button', { name: `Select “${RUNBOOK}”` }).click({ modifiers: ['Shift'] });
  const bar = page.getByRole('toolbar', { name: 'Selected action items' });
  await expect(bar).toBeVisible();
  await shoot(page, 'action-items-bulk');

  await bar.getByRole('button', { name: 'Assign' }).click();
  await page.getByRole('option', { name: /Maya Chen/ }).click();

  // No longer the user's: both leave "My items".
  await expect(page.getByRole('button', { name: JORDAN_DOC, exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: RUNBOOK, exact: true })).toHaveCount(0);
});

test("a meeting's items lead its Notes tab", async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  await page.goto(`/meetings/${oneOnOne.id}`);

  const block = page.getByRole('region', { name: /Action items/ });
  await expect(block).toBeVisible();
  await expect(block.getByText('8 open')).toBeVisible();
  await shoot(page, 'meeting-action-items');
});
