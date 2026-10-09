/**
 * Mock of `@/lib/ipc/generated/people`: @mention search over the fixtures'
 * attendees.
 */
import type * as Generated from '@/lib/ipc/generated/people';

import { mockPeople } from './database';
import { latency } from './latency';
import { ok } from './mock-database';
import { searchPeople } from './mock-people';

export const commands = {
  async search(query) {
    await latency();
    return ok(searchPeople(mockPeople, query));
  },
} satisfies typeof Generated.commands;
