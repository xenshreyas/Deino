/**
 * The ⌘K palette's provider registry (PARITY_PLAN §4.6, FND-10).
 *
 * A provider is a component that renders one `CommandGroup` of results for
 * the current search. Providers own their data (static lists, queries,
 * stores) and call `close()` after running an item. To add results, write a
 * provider and append it to `PALETTE_PROVIDERS` in `providers.ts`; ORG-08
 * adds full-text search this way.
 *
 * cmdk filters and ranks items by their `value` and `keywords`; a provider
 * whose results are already filtered (a search query) passes the search as
 * a keyword so cmdk keeps them.
 */
import type { ComponentType } from 'react';

import type { FilterChip } from '@/features/search';

/** What every provider receives. */
export interface PaletteProviderProps {
  /** The text in the palette input. */
  search: string;
  /** Closes the palette; call it after running an item. */
  close: () => void;
  /** The `@` filters picked so far (shown as chips in the input). */
  filters: readonly FilterChip[];
  /** Replaces the filters (a provider adds one, the chips remove one). */
  onFiltersChange: (filters: FilterChip[]) => void;
  /** Replaces the input's text (choosing an `@` filter removes the `@word`). */
  onSearchChange: (search: string) => void;
}

/** One source of palette results. */
export interface PaletteProvider {
  id: string;
  /** Groups render in ascending order; leave gaps of 10. */
  order: number;
  Component: ComponentType<PaletteProviderProps>;
}
