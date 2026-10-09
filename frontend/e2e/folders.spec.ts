import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('a new folder from the sidebar collects a meeting dragged onto it', async ({ page }) => {
  const interview = fixtureMeeting('meeting-fixture-interview');
  await page.goto('/meetings');
  const sidebar = page.getByRole('complementary', { name: 'Sidebar' });
  await expect(sidebar.getByRole('tree', { name: 'Folders' })).toBeVisible();

  await sidebar.getByRole('button', { name: 'New folder' }).click();
  const dialog = page.getByRole('dialog', { name: 'New folder' });
  await dialog.getByLabel('Name').fill('Hiring loop');
  await dialog.getByRole('button', { name: 'Create' }).click();
  const folder = sidebar.getByRole('treeitem', { name: 'Hiring loop' });
  await expect(folder).toBeVisible();

  await page.getByRole('link', { name: interview.title }).dragTo(folder);
  await expect(page.getByText('Added to Hiring loop')).toBeVisible();

  await folder.click();
  await expect(page.getByRole('heading', { name: 'Hiring loop' })).toBeVisible();
  await expect(page.getByRole('link', { name: interview.title })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('folder.png') });
});

test('a meeting from a recurring series offers to collect the series', async ({ page }) => {
  const standup = fixtureMeeting('meeting-fixture-standup');
  await page.goto('/meetings');
  const sidebar = page.getByRole('complementary', { name: 'Sidebar' });
  const customers = sidebar.getByRole('treeitem', { name: 'Customers' });
  await expect(customers).toBeVisible();

  await page.getByRole('link', { name: standup.title }).dragTo(customers);
  await expect(page.getByText('Auto-add future meetings in this series?')).toBeVisible();
  await page.getByRole('button', { name: 'Auto-add' }).click();
  await expect(page.getByText('Future meetings in this series will be added')).toBeVisible();

  await customers.click();
  await expect(page.getByText(`Every “${standup.title}”`)).toBeVisible();
});
