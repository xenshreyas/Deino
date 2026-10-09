/**
 * Vitest setup: a minimal Tauri runtime shim and Testing Library cleanup.
 *
 * `@tauri-apps/api` talks to the Rust core through `window.__TAURI_INTERNALS__`,
 * which only exists inside the Tauri webview. This shim lets modules that
 * import the API load and subscribe to events in happy-dom: every `invoke`
 * resolves to `null` (event subscriptions resolve to an id). Tests that need
 * real responses mock `@tauri-apps/api/core` with `vi.mock`.
 *
 * Adapted from anarlog `apps/desktop/src/test-setup.ts` (MIT, see
 * THIRD_PARTY_NOTICES.md).
 */
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

/** Callbacks registered through `transformCallback`, keyed `_<id>` like Tauri does. */
type CallbackRegistry = Record<string, unknown>;

/** Registers an IPC callback on `window` and returns its numeric id. */
function transformCallback(callback: unknown): number {
  const id = Math.trunc(Math.random() * Number.MAX_SAFE_INTEGER);
  (window as unknown as CallbackRegistry)[`_${id}`] = callback;
  return id;
}

/** Removes a callback registered by `transformCallback`. */
function unregisterCallback(id: number): void {
  Reflect.deleteProperty(window, `_${id}`);
}

Object.defineProperty(window, '__TAURI_INTERNALS__', {
  value: {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main' },
    },
    transformCallback: vi.fn(transformCallback),
    unregisterCallback: vi.fn(unregisterCallback),
    invoke: vi.fn((command: string) =>
      Promise.resolve(command === 'plugin:event|listen' ? 0 : null),
    ),
  },
  writable: true,
  configurable: true,
});

Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
  value: { unregisterListener: vi.fn() },
  writable: true,
  configurable: true,
});

// Vitest runs without globals, so Testing Library can't register this itself.
afterEach(() => {
  cleanup();
});
