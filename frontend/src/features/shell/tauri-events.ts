/**
 * Synchronous teardown for asynchronous Tauri event subscriptions.
 *
 * `listen()` resolves its unlisten function later, but React cleanups run
 * synchronously. This tracks pending subscriptions so a cleanup that runs
 * before they resolve still detaches them.
 */
import type { UnlistenFn } from '@tauri-apps/api/event';

/** Detaches every subscription in a group, including ones still pending. */
export type Unsubscribe = () => void;

/**
 * Takes ownership of pending `listen()` calls and returns one cleanup for all.
 *
 * A subscription that fails (for example in a plain browser without Tauri) is
 * logged and skipped, so one missing event never breaks the others.
 */
export function holdSubscriptions(pending: readonly Promise<UnlistenFn>[]): Unsubscribe {
  let disposed = false;
  const active: UnlistenFn[] = [];
  for (const subscription of pending) {
    subscription
      .then((unlisten) => {
        if (disposed) unlisten();
        else active.push(unlisten);
      })
      .catch((error: unknown) => {
        console.error('[shell] Tauri event subscription failed', error);
      });
  }
  return () => {
    disposed = true;
    active.splice(0).forEach((unlisten) => {
      unlisten();
    });
  };
}
