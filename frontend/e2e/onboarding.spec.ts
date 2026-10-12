import { expect, type Page, test } from '@playwright/test';

/**
 * First-run setup (FND-11) over `dev:mock`. `/?onboarding` starts a fresh
 * install's flow; the mock keeps its settings in memory, so each test moves
 * through the app without reloading.
 */

/**
 * Waits until every finite animation and colour transition has ended or been
 * cancelled. A script string, because the e2e config has no DOM types.
 */
const SETTLE = `Promise.allSettled(
  document
    .getAnimations()
    .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
    .map((animation) => animation.finished),
)`;

/** Saves a screenshot of the page in the light and the dark theme. */
async function shoot(page: Page, name: string) {
  // Off every control, so no hover state lands in the picture.
  await page.mouse.move(0, 0);
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.evaluate(SETTLE);
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

/**
 * Waits for the step called `title`, its saved move and `loaded` (text that
 * replaces the step's skeleton), then screenshots it.
 */
async function atStep(page: Page, title: string, shot: string, loaded?: string) {
  await expect(page.getByRole('heading', { level: 1, name: title })).toBeVisible();
  await expect(page.getByRole('contentinfo').getByRole('button').last()).toBeEnabled();
  if (loaded !== undefined) await expect(page.getByText(loaded).first()).toBeVisible();
  await shoot(page, shot);
}

/** Clicks a footer button: Continue, Skip, Back, Finish. */
async function press(page: Page, name: string) {
  await page.getByRole('contentinfo').getByRole('button', { name, exact: true }).click();
}

/** The app's shell, which replaces onboarding once it is finished. */
function sidebar(page: Page) {
  return page.getByRole('complementary', { name: 'Sidebar' });
}

test('a fresh install walks all five steps, skipping some, and lands in the app', async ({
  page,
}) => {
  await page.goto('/?onboarding');

  await atStep(page, 'Welcome to Deino', '1-welcome');
  await page.getByRole('button', { name: 'Get started' }).click();

  await atStep(page, 'Permissions', '2-permissions', 'Microphone');
  // UX-21, UX-22: only what recording needs; Calendar has its own step.
  await expect(page.getByRole('list', { name: 'Permissions' }).getByRole('listitem')).toHaveCount(
    2,
  );
  const systemAudio = page.getByRole('listitem').filter({ hasText: 'System audio' });
  await systemAudio.getByRole('button', { name: 'Allow' }).click();
  await expect(systemAudio.getByText('Allowed')).toBeVisible();
  await press(page, 'Continue');

  await atStep(page, 'AI setup', '3-ai-setup', 'Transcription is ready');
  await press(page, 'Skip');

  await atStep(page, 'Calendar', '4-calendar', 'Sync calendars');
  await press(page, 'Skip');

  await atStep(page, 'About you', '5-profile');
  await page.getByLabel('Company').fill('Deino');
  await press(page, 'Continue');

  await expect(page.getByRole('heading', { name: 'Welcome to Deino' })).toHaveCount(0);
  await expect(sidebar(page)).toBeVisible();
});

test('Back returns to the previous step, and Skip setup goes straight to the app', async ({
  page,
}) => {
  await page.goto('/?onboarding');
  await page.getByRole('button', { name: 'Get started' }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'Permissions' })).toBeVisible();
  await press(page, 'Back');
  await expect(page.getByRole('heading', { level: 1, name: 'Welcome to Deino' })).toBeVisible();

  await page.getByRole('button', { name: 'Skip setup' }).click();

  await expect(sidebar(page)).toBeVisible();
});

test('setup ends after About you, with no Try it step (UX-24)', async ({ page }) => {
  await page.goto('/?onboarding');
  await page.getByRole('button', { name: 'Get started' }).click();
  for (const title of ['Permissions', 'AI setup', 'Calendar', 'About you']) {
    await expect(page.getByRole('heading', { level: 1, name: title })).toBeVisible();
    await expect(page.getByRole('list', { name: /Step \d of 5/ })).toBeVisible();
    await press(page, 'Skip');
  }

  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole('heading', { level: 1, name: 'Try it' })).toHaveCount(0);
});

test('Settings → Setup resumes at the first step left undone', async ({ page }) => {
  await page.goto('/?onboarding');
  await page.getByRole('button', { name: 'Get started' }).click();
  await press(page, 'Continue');
  await expect(page.getByRole('heading', { level: 1, name: 'AI setup' })).toBeVisible();
  await press(page, 'Skip');
  for (const title of ['Calendar', 'About you']) {
    await expect(page.getByRole('heading', { level: 1, name: title })).toBeVisible();
    await press(page, 'Continue');
  }

  await sidebar(page).getByRole('link', { name: 'Settings', exact: true }).click();
  await page
    .getByRole('navigation', { name: 'Settings pages' })
    .getByRole('link', { name: 'General' })
    .click();
  await expect(page.getByText('Not done yet: AI setup.')).toBeVisible();
  await shoot(page, 'settings-setup');
  await page.getByRole('button', { name: 'Resume setup' }).click();

  await expect(page.getByRole('heading', { level: 1, name: 'AI setup' })).toBeVisible();
});

test('a finished user never sees onboarding', async ({ page }) => {
  await page.goto('/');

  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Welcome to Deino' })).toHaveCount(0);
  await page.reload();
  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Welcome to Deino' })).toHaveCount(0);
});
