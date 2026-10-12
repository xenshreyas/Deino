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
const BOARD = fixtureMeeting('meeting-fixture-board-update');
const STANDUP = fixtureMeeting('meeting-fixture-standup');

/** The provider steps the picker lists as "WIP" (PARITY_PLAN D12). */
const WIP_STEPS = [
  'Webhook',
  'Slack',
  'Notion',
  'Task tools',
  'CRM',
  'Google Docs or Apple Notes',
  'Email',
];

/** The Automations page's list of automations. */
const rail = (page: Page) => page.getByRole('navigation', { name: 'Automations', exact: true });

/** Adds the step named `name` from the "Add step" menu. */
async function addStep(page: Page, name: string) {
  await page.getByRole('button', { name: 'Add step' }).click();
  await page.getByRole('menuitem', { name, exact: true }).click();
}

/** Builds "Discovery recap": two condition groups, a notification and an export. */
async function buildRecap(page: Page) {
  await page.goto('/automations');
  await expect(page.getByRole('heading', { name: 'No automations yet' })).toBeVisible();
  await page.getByRole('button', { name: 'New automation' }).first().click();
  // While creating, the draft heads the rail as the current entry.
  await expect(rail(page).getByRole('button', { name: /^New automation/ })).toHaveAttribute(
    'aria-current',
    'page',
  );

  await page.getByRole('textbox', { name: 'Name' }).fill('Discovery recap');
  await page.getByRole('button', { name: 'Add condition' }).click();
  await page.getByRole('textbox', { name: 'Title contains' }).fill('discovery');
  await page.getByRole('button', { name: 'Or another group' }).click();
  await page.getByRole('textbox', { name: 'Title contains' }).nth(1).fill('board');

  await addStep(page, 'Notification');
  await page.getByRole('textbox', { name: 'Message (optional)' }).fill('Recap ready');
  await addStep(page, 'Export file');
  await page.getByRole('button', { name: 'Choose folder…' }).click();
  await expect(page.getByRole('group', { name: 'Export folder' })).toContainText('Meeting exports');
  await shoot(page, 'automations-builder');
  await page.setViewportSize({ width: 760, height: 900 });
  await shoot(page, 'automations-builder-narrow');
  await page.setViewportSize({ width: 1280, height: 720 });

  await page.getByRole('button', { name: 'Save' }).click();
  await expect(
    rail(page).getByRole('button', { name: 'Discovery recap', exact: true }),
  ).toHaveAttribute('aria-current', 'page');
}

test('an automation with conditions and two steps saves, duplicates, runs and reruns', async ({
  page,
}) => {
  await buildRecap(page);

  // Duplicate: a switched-off copy, selected.
  await page.getByRole('button', { name: 'More actions' }).click();
  await page.getByRole('menuitem', { name: 'Duplicate' }).click();
  const copy = rail(page).getByRole('button', { name: /^Discovery recap \(copy\)/ });
  await expect(copy).toHaveAttribute('aria-current', 'page');
  await expect(copy).toContainText('Off');
  await rail(page).getByRole('button', { name: 'Discovery recap', exact: true }).click();

  // Matching meetings → Run on selected → the run's status.
  await page.getByRole('tab', { name: 'Matching meetings' }).click();
  const sales = page.getByRole('checkbox', { name: SALES.title });
  await expect(sales).toBeVisible();
  await expect(page.getByRole('checkbox', { name: BOARD.title })).toBeVisible();
  await expect(page.getByRole('checkbox', { name: STANDUP.title })).toHaveCount(0);
  await sales.check();
  await page.getByRole('button', { name: 'Run on 1 selected' }).click();
  const salesRow = page.getByRole('listitem').filter({ has: sales });
  await expect(salesRow).toContainText('Done', { timeout: 10_000 });
  await shoot(page, 'automations-matching');

  // The builder's run log shows the run and each step's result.
  await page.getByRole('tab', { name: 'Builder' }).click();
  const log = page.getByRole('list', { name: 'Recent runs' });
  await expect(log).toContainText('Export file · Done');
  await log.scrollIntoViewIfNeeded();
  await shoot(page, 'automations-run-log');

  await log.getByRole('link', { name: `Open ${SALES.title}` }).click();
  await rerunFromMeeting(page);
});

/** On the sales meeting: ⋯ → Automations shows each run's status, and Rerun runs it again. */
async function rerunFromMeeting(page: Page) {
  await expect(page).toHaveURL(/\/meetings\/meeting-fixture-sales-discovery/);
  await expect(page.getByRole('heading', { name: SALES.title, level: 1 })).toBeVisible();
  // The header settles as the meeting's queries land; open the menu once it holds.
  await expect(async () => {
    await page.getByRole('button', { name: 'More actions' }).click();
    await expect(page.getByRole('menuitem', { name: 'Automations' })).toBeVisible({
      timeout: 1_000,
    });
  }).toPass();
  await page.getByRole('menuitem', { name: 'Automations' }).click();
  const rerun = page.getByRole('menuitem').filter({ hasText: 'Rerun' });
  await expect(rerun).toContainText('Discovery recap');
  await expect(rerun).toContainText('Done');
  await expect(
    page.getByRole('menuitem').filter({ hasText: 'Discovery recap (copy)' }),
  ).toContainText('Not run');
  await shoot(page, 'meeting-automations-menu');
  await rerun.click();
  await expect(rerun).toContainText('Done', { timeout: 10_000 });
}

test('a script step is blocked until shell scripts are allowed; provider steps are WIP', async ({
  page,
}) => {
  await page.goto('/automations?automation=new');
  const allow = page.getByRole('switch', { name: 'Allow shell scripts' });
  await expect(allow).not.toBeChecked();

  await page.getByRole('button', { name: 'Add step' }).click();
  const shell = page.getByRole('menuitem', { name: 'Run shell script', exact: true });
  await expect(shell).toBeDisabled();
  await expect(shell).toHaveAccessibleDescription(/Allow shell scripts/);
  await expect(page.getByRole('menuitem', { name: 'Notification', exact: true })).toBeEnabled();
  for (const name of WIP_STEPS) {
    await expect(page.getByRole('menuitem', { name, exact: true })).toBeDisabled();
  }
  await shoot(page, 'automations-step-picker');
  await page.keyboard.press('Escape');

  await allow.click();
  await expect(allow).toBeChecked();
  await addStep(page, 'Run shell script');
  await page.getByRole('textbox', { name: 'Command' }).fill('cat > payload.json');
  await expect(page.getByText(/Shell scripts are off/)).toHaveCount(0);

  // Turning scripts off again keeps the step but says it won't run.
  await allow.click();
  await expect(page.getByText(/Shell scripts are off/)).toBeVisible();
  await shoot(page, 'automations-shell-off');
});
