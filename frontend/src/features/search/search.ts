/**
 * URL search parameters of `/search`, validated with zod: the text, the tab
 * and the `@` filters. Malformed values fall back to "absent".
 */
import { z } from 'zod';

/** The results page's tabs. */
export const searchTabSchema = z.enum(['meetings', 'moments', 'people']);

/** One of the results page's tabs. */
export type SearchTab = z.infer<typeof searchTabSchema>;

const filterChipSchema = z.object({
  kind: z.enum(['person', 'company', 'tag', 'folder', 'meeting']),
  id: z.string().min(1),
  label: z.string(),
});

/** `/search?q=roadmap&tab=moments&filters=[…]` */
export const searchPageSchema = z.object({
  q: z.string().optional().catch(undefined),
  tab: searchTabSchema.optional().catch(undefined),
  filters: z.array(filterChipSchema).optional().catch(undefined),
});

/** The validated `/search` parameters. */
export type SearchPageParams = z.infer<typeof searchPageSchema>;
