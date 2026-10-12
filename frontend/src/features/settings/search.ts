/**
 * URL search parameters of `/settings`: which page is open. Unknown values
 * fall back to the first page.
 */
import { z } from 'zod';

import { isSettingsSectionId } from './sections';

/**
 * Every settings page's id, as keys. Each page adds its own from its
 * feature's `settings-page.tsx` (REG-02), so links to a page are checked:
 *
 * ```ts
 * declare module '@/features/settings/search' {
 *   interface SettingsPageIds {
 *     screen: true;
 *   }
 * }
 * ```
 *
 * `general` is declared here: the page that opens when none is named.
 */
export interface SettingsPageIds {
  general: true;
}

/** One settings page. */
export type SettingsSectionId = keyof SettingsPageIds;

/** A registered page's id. */
export const settingsSectionSchema = z.custom<SettingsSectionId>(isSettingsSectionId);

/**
 * `/settings?section=appearance`. `template` opens a template in the
 * Templates page's editor (`new` for a new one).
 */
export const settingsSearchSchema = z.object({
  section: settingsSectionSchema.optional().catch(undefined),
  template: z.string().min(1).optional().catch(undefined),
});
