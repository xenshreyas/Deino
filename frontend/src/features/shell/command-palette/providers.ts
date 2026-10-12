/**
 * Every palette provider, in display order. Append-only: a feature adds its
 * provider here (see `registry.ts`).
 */
import { ClipsPalette } from '@/features/clips';
import { FoldersPalette } from '@/features/folders';
import { FilterPalette, SearchPalette } from '@/features/search';
import { TagsPalette } from '@/features/tags';

import { ActionsProvider } from './actions-provider';
import { GoToProvider } from './go-to-provider';
import { MeetingsProvider } from './meetings-provider';
import type { PaletteProvider } from './registry';

export const PALETTE_PROVIDERS: readonly PaletteProvider[] = [
  { id: 'search-filters', order: 5, Component: FilterPalette },
  { id: 'meetings', order: 10, Component: MeetingsProvider },
  { id: 'search', order: 20, Component: SearchPalette },
  { id: 'folders', order: 24, Component: FoldersPalette },
  { id: 'tags', order: 26, Component: TagsPalette },
  { id: 'clips', order: 28, Component: ClipsPalette },
  { id: 'actions', order: 30, Component: ActionsProvider },
  { id: 'go-to', order: 40, Component: GoToProvider },
];
