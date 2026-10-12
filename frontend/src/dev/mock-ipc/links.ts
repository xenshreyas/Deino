/**
 * Mock of `@/lib/ipc/generated/links`: no `deino://` link ever arrives in a
 * plain browser, so nothing is ever waiting.
 */
import type * as Generated from '@/lib/ipc/generated/links';

import { ok } from './command-result';
import { latency } from './latency';
import { mockEvent } from './mock-event';

export const events = {
  navigationWaiting: mockEvent<Generated.NavigationWaiting>('plugin:links:navigation-waiting'),
  linkRejected: mockEvent<Generated.LinkRejected>('plugin:links:link-rejected'),
} satisfies typeof Generated.events;

export const commands = {
  async take() {
    await latency();
    return ok<Generated.Navigation[]>([]);
  },
} satisfies typeof Generated.commands;
