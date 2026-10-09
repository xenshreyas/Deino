/**
 * Mock of `@/lib/ipc/generated/meetings`, served from the fixture meetings.
 * `satisfies` keeps it in step with the Rust commands: a changed signature
 * fails `pnpm typecheck` here.
 */
import type * as Generated from '@/lib/ipc/generated/meetings';

import { mockDatabase } from './database';
import { latency } from './latency';
import { ok } from './mock-database';

export const commands = {
  async list() {
    await latency();
    return ok(mockDatabase.meetings());
  },
  async get(id) {
    await latency();
    return mockDatabase.meeting(id);
  },
  async rename(id, title) {
    await latency();
    return mockDatabase.rename(id, title, new Date());
  },
} satisfies typeof Generated.commands;
