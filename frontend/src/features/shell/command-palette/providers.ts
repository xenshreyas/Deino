/**
 * Every palette provider, in display order. Append-only: a feature adds its
 * provider here (see `registry.ts`).
 */
import { ActionsProvider } from './actions-provider';
import { GoToProvider } from './go-to-provider';
import { MeetingsProvider, TranscriptsProvider } from './meetings-provider';
import type { PaletteProvider } from './registry';

export const PALETTE_PROVIDERS: readonly PaletteProvider[] = [
  { id: 'meetings', order: 10, Component: MeetingsProvider },
  { id: 'transcripts', order: 20, Component: TranscriptsProvider },
  { id: 'actions', order: 30, Component: ActionsProvider },
  { id: 'go-to', order: 40, Component: GoToProvider },
];
