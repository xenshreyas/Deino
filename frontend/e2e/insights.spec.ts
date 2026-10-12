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

const SALES = fixtureMeeting('meeting-fixture-sales-discovery');

/** Creates "Discovery scorecard" from the gallery and runs it on the matching meetings. */
async function createAndRun(page: Page) {
  await page.goto('/insights');
  await expect(page.getByRole('heading', { name: 'No insights yet' })).toBeVisible();
  await shoot(page, 'insights-empty');

  await page.getByRole('button', { name: 'Start from a template' }).click();
  const gallery = page.getByRole('dialog', { name: 'New insight' });
  await shoot(page, 'insights-gallery');
  await gallery.getByRole('button', { name: /^Discovery scorecard/ }).click();

  const editor = page.getByRole('dialog', { name: 'New insight' });
  await expect(editor.getByRole('textbox', { name: 'Name', exact: true })).toHaveValue(
    'Discovery scorecard',
  );
  await shoot(page, 'insights-editor');
  await editor.getByRole('button', { name: 'Create insight' }).click();

  await expect(page.getByRole('heading', { name: 'Discovery scorecard', level: 2 })).toBeVisible();
  await page.getByRole('button', { name: 'Run on past meetings' }).first().click();
  const run = page.getByRole('dialog', { name: /Run “Discovery scorecard” on past meetings/ });
  await expect(run.getByRole('checkbox').first()).toBeVisible();
  await shoot(page, 'insights-run');
  await run.getByRole('button', { name: /^Run on \d+ meetings?$/ }).click();
}

test('an insight from a template answers on a meeting, fills its table and exports CSV', async ({
  page,
}) => {
  await createAndRun(page);

  const table = page.getByRole('table', { name: 'Discovery scorecard answers' });
  const salesRow = table.getByRole('link', { name: SALES.title });
  await expect(salesRow).toBeVisible({ timeout: 15_000 });
  await expect(page.getByRole('status')).toHaveCount(0, { timeout: 15_000 });
  await shoot(page, 'insights-table');

  // The meeting's notes end with the same answer, attributed.
  await salesRow.click();
  await expect(page).toHaveURL(/\/meetings\/meeting-fixture-sales-discovery\?tab=notes/);
  const section = page.getByRole('region', { name: 'Discovery scorecard' });
  await expect(section).toBeVisible();
  await expect(section.getByRole('link', { name: /where Marcus Webb said this/ })).toBeVisible();
  await shoot(page, 'meeting-insights');

  // Back on the Insights page, the table exports as CSV.
  await page.getByRole('link', { name: 'Insights' }).first().click();
  await page.getByRole('button', { name: 'Export CSV' }).click();
  await expect(page.getByText('Saved Discovery scorecard.csv')).toBeVisible();
});

test('clicking a column header sorts the table', async ({ page }) => {
  await createAndRun(page);
  const table = page.getByRole('table', { name: 'Discovery scorecard answers' });
  await expect(table.getByRole('row')).not.toHaveCount(1, { timeout: 15_000 });

  await table.getByRole('button', { name: 'Sort by Meeting' }).click();

  await expect(table.getByRole('columnheader', { name: /Meeting/ })).toHaveAttribute(
    'aria-sort',
    'ascending',
  );
});
