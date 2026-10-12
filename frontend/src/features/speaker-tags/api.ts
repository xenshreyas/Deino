/**
 * Typed calls to the `speaker-tags` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/speaker-tags';
import { unwrap } from '@/lib/ipc/result';

import type { AppTagStatus } from './types';

export const speakerTagsApi = {
  /** One row per meeting app: Zoom, Teams, Meet. */
  status: (): Promise<AppTagStatus[]> => unwrap(commands.status()),
  /** Opens System Settings → Privacy & Security → Accessibility. */
  openAccessibilitySettings: (): Promise<null> => unwrap(commands.openAccessibilitySettings()),
};
