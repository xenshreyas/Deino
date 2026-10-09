import { describe, expect, test, vi } from 'vitest';

import { holdSubscriptions } from './tauri-events';

/** Lets pending promise callbacks run. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe('holdSubscriptions', () => {
  test('cleanup detaches subscriptions that already resolved', async () => {
    const unlisten = vi.fn();
    const unsubscribe = holdSubscriptions([Promise.resolve(unlisten)]);
    await settle();

    unsubscribe();

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  test('a subscription resolving after cleanup is detached immediately (no leaked listener)', async () => {
    const unlisten = vi.fn();
    let resolveListen!: (value: () => void) => void;
    const unsubscribe = holdSubscriptions([new Promise((resolve) => (resolveListen = resolve))]);

    unsubscribe();
    resolveListen(unlisten);
    await settle();

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  test('a failed subscription does not stop the others from being detached', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const unlisten = vi.fn();
    const unsubscribe = holdSubscriptions([
      Promise.reject(new Error('no Tauri')),
      Promise.resolve(unlisten),
    ]);
    await settle();

    unsubscribe();

    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(consoleError).toHaveBeenCalledTimes(1);
    consoleError.mockRestore();
  });
});
