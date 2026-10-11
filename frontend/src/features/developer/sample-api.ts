/**
 * Typed calls to the debug-only `dev` plugin. Only the lazily loaded sample
 * meetings panel imports this, so release builds never include it.
 */
import { commands } from '@/lib/ipc/generated/dev';
import { unwrap } from '@/lib/ipc/result';

import type { SeedReport } from './types';

export const sampleApi = {
  /** Writes the fixture meetings into the database and reports what it wrote. */
  seedFixtures: (): Promise<SeedReport> => unwrap(commands.seedFixtures()),
};
