/**
 * Mock of `@/lib/ipc/generated/speaker-tags`: every app needs Accessibility
 * until "Open System Settings" is pressed, which grants it, so `pnpm
 * dev:mock` shows both states. `?speakerTags=enabled` or `=unsupported`
 * starts in that state instead (screenshots).
 *
 * It imports no feature code: the feature reaches this module through its
 * generated bindings, so importing it back would form a cycle.
 */
import type * as Generated from '@/lib/ipc/generated/speaker-tags';

import { ok } from './command-result';
import { latency } from './latency';

/** Every app, in the order Rust lists them. */
const APPS: readonly Generated.CallApp[] = ['zoom', 'teams', 'meet'];

/** The state the page asked for (`?speakerTags=`), "needs permission" by default. */
function requestedState(): Generated.TagState {
  const state = new URLSearchParams(window.location.search).get('speakerTags');
  return state === 'enabled' || state === 'unsupported' ? state : 'needsPermission';
}

let state: Generated.TagState | null = null;

export const commands = {
  async status() {
    await latency();
    state ??= requestedState();
    const current = state;
    return ok(APPS.map((app) => ({ app, state: current })));
  },
  async openAccessibilitySettings() {
    await latency();
    if (state === 'needsPermission') state = 'enabled';
    return ok(null);
  },
} satisfies typeof Generated.commands;
