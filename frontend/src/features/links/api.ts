/**
 * Typed calls to the `links` plugin. Each resolves to the data or throws an
 * `IpcError`; the link host is the only caller.
 */
import { commands } from '@/lib/ipc/generated/links';
import { unwrap } from '@/lib/ipc/result';

import type { Navigation } from './types';

export const linksApi = {
  /**
   * The navigations `deino://` links and hotkeys queued, oldest first; taking
   * them empties the queue. Refused while Deino is locked (PRV-02).
   */
  take: (): Promise<Navigation[]> => unwrap(commands.take()),
};
