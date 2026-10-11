/**
 * Mock of `@/lib/ipc/generated/dev`: seeding resets the mock database to the
 * fixtures, like `FixtureSeeder` does for a real one.
 */
import type * as Generated from '@/lib/ipc/generated/dev';

import { mockDatabase, mockEnhanced, mockEventLinks, mockNotes } from './database';
import { latency } from './latency';
import { ok } from './mock-database';

export const commands = {
  async seedFixtures() {
    await latency();
    mockNotes.reseed();
    mockEnhanced.reseed();
    mockEventLinks.reseed();
    return ok(mockDatabase.reseed());
  },
} satisfies typeof Generated.commands;
