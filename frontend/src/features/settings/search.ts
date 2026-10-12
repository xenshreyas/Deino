/**
 * URL search parameters of `/settings`: which page is open. Unknown values
 * fall back to the first page.
 */
import { z } from 'zod';

/** Every settings page. */
export const settingsSectionSchema = z.enum([
  'general',
  'appearance',
  'shortcuts',
  'setup',
  'recording',
  'detection',
  'transcription',
  'vocabulary',
  'watched-folders',
  'people',
  'speaker-tags',
  'consent',
  'ai',
  'models',
  'templates',
  'profile',
  'search',
  'calendar',
  'integrations',
  'security',
  'network',
  'export',
  'beta',
  'about',
  'developer',
]);

/** One settings page. */
export type SettingsSectionId = z.infer<typeof settingsSectionSchema>;

/**
 * `/settings?section=appearance`. `template` opens a template in the
 * Templates page's editor (`new` for a new one).
 */
export const settingsSearchSchema = z.object({
  section: settingsSectionSchema.optional().catch(undefined),
  template: z.string().min(1).optional().catch(undefined),
});
