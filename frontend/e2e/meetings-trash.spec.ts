/**
 * The Meetings page's trash and merge (ORG-07): a trashed meeting leaves the
 * list, waits in the Trash and comes back whole; two meetings merge into
 * the earliest, which opens.
 */
import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';
import { shootBothThemes } from './themes';

const interview = fixtureMeeting('meeting-fixture-interview');
const standup = fixtureMeeting('meeting-fixture-standup');
const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');

/** An item of the interview the action items page lists (overdue in the fixtures). */
const JORDAN_DOC = 'Send Jordan the doc on how on-call and remote work at Deino';

const row = (page: Page, title: string) => page.getByRole('link', { name: title });
const sidebar = (page: Page) => page.getByRole('complementary', { name: 'Sidebar' });

/** Files the interview into Customers through the bulk bar. */
async function fileInterview(page: Page) {
  await page.getByRole('checkbox', { name: `Select ${interview.title}`, exact: true }).click();
  const bar = page.getByRole('toolbar', { name: 'Selected meetings' });
  await bar.getByRole('button', { name: 'Add to folder' }).click();
  await page.getByRole('option', { name: 'Customers', exact: true }).click();
  // UX-32: filing is silent; the rest of the test checks the folder.
  await page.keyboard.press('Escape');
}

test.beforeEach(async ({ page }) => {
  await page.goto('/meetings');
  await expect(row(page, standup.title)).toBeVisible();
});

test('a trashed meeting comes back with its notes, tags, folders and action items', async ({
  page,
}) => {
  await fileInterview(page);
  await page.getByRole('button', { name: `${interview.title} actions` }).click();
  await page.getByRole('menuitem', { name: 'Delete' }).click();
  await expect(page.getByText('Moved 1 meeting to the trash')).toBeVisible();
  await expect(row(page, interview.title)).toBeHidden();

  // UX-25: Trash lives in the Meetings header, not the sidebar.
  await page.getByRole('main').getByRole('link', { name: 'Trash' }).click();
  await expect(page.getByRole('heading', { name: 'Trash' })).toBeVisible();
  const trashed = page.getByRole('listitem').filter({ hasText: interview.title });
  await expect(trashed).toContainText('Deleted in 30 days');
  await shootBothThemes(page, 'meetings-trash');
  await trashed.getByRole('button', { name: 'Restore' }).click();
  await expect(page.getByText('The trash is empty')).toBeVisible();

  // In-app navigation: a reload would reset the mock's state.
  await sidebar(page).getByRole('link', { name: 'Meetings' }).click();
  await expect(row(page, interview.title)).toContainText('Hiring');
  await sidebar(page).getByRole('treeitem', { name: 'Customers' }).click();
  await expect(row(page, interview.title)).toBeVisible();
  await row(page, interview.title).click();
  await expect(page.getByRole('heading', { name: interview.title })).toBeVisible();
  await expect(page.getByText('webhook = hint, cursor = truth')).toBeVisible();
  await sidebar(page).getByRole('link', { name: 'Action items' }).click();
  await expect(page.getByRole('button', { name: JORDAN_DOC, exact: true })).toBeVisible();
});

test('Undo on the toast takes a deleted meeting straight back', async ({ page }) => {
  await page.getByRole('button', { name: `${standup.title} actions` }).click();
  await page.getByRole('menuitem', { name: 'Delete' }).click();
  await expect(row(page, standup.title)).toBeHidden();

  await page.getByRole('button', { name: 'Undo' }).click();

  await expect(row(page, standup.title)).toBeVisible();
});

test('two meetings merge into the earliest, which opens', async ({ page }) => {
  await page.getByRole('checkbox', { name: `Select ${standup.title}`, exact: true }).click();
  await page.getByRole('checkbox', { name: `Select ${oneOnOne.title}`, exact: true }).click();
  await page
    .getByRole('toolbar', { name: 'Selected meetings' })
    .getByRole('button', { name: 'Merge' })
    .click();
  const dialog = page.getByRole('dialog', { name: 'Merge 2 meetings?' });
  await expect(dialog).toBeVisible();
  await shootBothThemes(page, 'meetings-merge');

  await dialog.getByRole('button', { name: 'Merge' }).click();

  await expect(page.getByRole('heading', { name: oneOnOne.title })).toBeVisible();
  await sidebar(page).getByRole('link', { name: 'Meetings' }).click();
  await expect(row(page, oneOnOne.title)).toBeVisible();
  await expect(row(page, standup.title)).toBeHidden();
});
