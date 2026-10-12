/**
 * Mock of `@/lib/ipc/generated/links`: no `deino://` link ever arrives in a
 * plain browser, so nothing is waiting. A test or the console can queue one
 * with {@link queueMockNavigation}, which also emits `navigation-waiting`.
 */
import type * as Generated from '@/lib/ipc/generated/links';

import { ok } from './command-result';
import { latency } from './latency';
import { mockEvent } from './mock-event';

let waiting: Generated.Navigation[] = [];

export const events = {
  navigationWaiting: mockEvent<Generated.NavigationWaiting>('plugin:links:navigation-waiting'),
  linkRejected: mockEvent<Generated.LinkRejected>('plugin:links:link-rejected'),
} satisfies typeof Generated.events;

/** Queues `navigation` as a link would, and tells the window. */
export function queueMockNavigation(navigation: Generated.Navigation): Promise<void> {
  waiting.push(navigation);
  return events.navigationWaiting.emit({});
}

export const commands = {
  async take() {
    await latency();
    const taken = waiting;
    waiting = [];
    return ok(taken);
  },
} satisfies typeof Generated.commands;
