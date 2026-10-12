/**
 * URL search parameters of `/automations`, validated with zod:
 * `?automation=<id>` selects an automation, `?automation=new` opens a blank
 * builder (absent: the first). Malformed values fall back to "absent",
 * because the URLs come from the app itself.
 */
import { z } from 'zod';

/** The value that opens a new, blank automation. */
export const NEW_AUTOMATION = 'new';

/** `/automations?automation=<id|new>` */
export const automationsSearchSchema = z.object({
  automation: z.string().min(1).optional().catch(undefined),
});

/** The validated search parameters. */
export type AutomationsSearch = z.infer<typeof automationsSearchSchema>;
