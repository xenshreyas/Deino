/**
 * URL search parameters of `/settings`: which page is open. Unknown values
 * fall back to the first page.
 */
import { z } from 'zod';

/** Every settings page. */
export const settingsSectionSchema = z.enum([
  'general',
  'appearance',
  'recording',
  'transcription',
  'models',
  'beta',
  'about',
  'developer',
]);

/** One settings page. */
export type SettingsSectionId = z.infer<typeof settingsSectionSchema>;

/** `/settings?section=appearance` */
export const settingsSearchSchema = z.object({
  section: settingsSectionSchema.optional().catch(undefined),
});
