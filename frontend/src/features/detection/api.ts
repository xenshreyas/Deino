/**
 * Typed calls to the `detection` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/detection';
import { unwrap } from '@/lib/ipc/result';

import type { Detection, DetectionId, DetectionOverview } from './types';

export const detectionApi = {
  /** The prompt on screen, or `null`. */
  current: (): Promise<Detection | null> => unwrap(commands.current()),
  /** "Take notes": closes the prompt and starts recording; `not_found` once it closed. */
  takeNotes: (id: DetectionId): Promise<null> => unwrap(commands.takeNotes(id)),
  /** "Ignore" / "Don't record": closes the prompt; the app stays quiet for ten minutes. */
  dismiss: (id: DetectionId): Promise<null> => unwrap(commands.dismiss(id)),
  /** "Always ignore this app": closes the prompt and adds the app to the ignore list. */
  ignoreApp: (id: DetectionId): Promise<null> => unwrap(commands.ignoreApp(id)),
  /** The detection settings and what this Mac supports. */
  overview: (): Promise<DetectionOverview> => unwrap(commands.settings()),
};
