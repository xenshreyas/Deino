/**
 * Mock of `@/lib/ipc/generated/db`: the `DbChanged` event, emitted by the
 * mock database after every write.
 */
import type * as Generated from '@/lib/ipc/generated/db';

import { mockEvent } from './mock-event';

export const commands = {} satisfies typeof Generated.commands;

export const events = {
  dbChanged: mockEvent<Generated.DbChanged>('plugin:db:db-changed'),
} satisfies typeof Generated.events;
