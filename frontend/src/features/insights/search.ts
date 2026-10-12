/**
 * URL search parameters of `/insights`, validated with zod:
 * `?insight=<id>` selects an insight (absent: the first). Malformed values
 * fall back to "absent", because the URLs come from the app itself.
 */
import { z } from 'zod';

/** `/insights?insight=<id>` */
export const insightsSearchSchema = z.object({
  insight: z.string().min(1).optional().catch(undefined),
});

/** The validated search parameters. */
export type InsightsSearch = z.infer<typeof insightsSearchSchema>;
