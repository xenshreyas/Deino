/**
 * URL search parameters of the Meetings page, validated with zod.
 *
 * `tags` filters the list to meetings carrying any of the tags (ORG-06).
 * Malformed values fall back to "absent" instead of failing the route.
 */
import { z } from 'zod';

/** `/meetings?tags=[...]` */
export const meetingsSearchSchema = z.object({
  tags: z.array(z.string().min(1)).optional().catch(undefined),
});

/** The Meetings page's search parameters. */
export type MeetingsSearch = z.infer<typeof meetingsSearchSchema>;
