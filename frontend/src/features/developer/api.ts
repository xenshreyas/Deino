/**
 * Typed calls to the debug-only `dev` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/dev';
import { unwrap } from '@/lib/ipc/result';

import type { SeedReport } from './types';

export const developerApi = {
  /** Writes the fixture meetings into the database and reports what it wrote. */
  seedFixtures: (): Promise<SeedReport> => unwrap(commands.seedFixtures()),
};
