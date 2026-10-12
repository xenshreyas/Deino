/** Screenshots in both themes, for a UI card's PR (PARITY_PLAN §3B). */
import { type Page, test } from '@playwright/test';

/** Captures the page as `<name>-light.png` and `<name>-dark.png`, then returns to light. */
export async function shootBothThemes(page: Page, name: string): Promise<void> {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the theme's colour transitions finish before the capture.
    await page.waitForTimeout(400);
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}
