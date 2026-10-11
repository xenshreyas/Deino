/**
 * URL search parameters of `/action-items`, validated with zod:
 * `?view=<saved view id>` opens a saved view's tab (absent: "My items"),
 * `?add=true` focuses quick add (⌘K "New action item"). Malformed values
 * fall back to "absent", because the URLs come from the app itself.
 */
import { z } from 'zod';

/** `/action-items?view=<id>&add=true` */
export const actionItemsSearchSchema = z.object({
  view: z.string().min(1).optional().catch(undefined),
  add: z.boolean().optional().catch(undefined),
});

/** The validated search parameters. */
export type ActionItemsSearch = z.infer<typeof actionItemsSearchSchema>;
