/**
 * Turns a plain browser tab into a mock Tauri webview for `pnpm dev:mock`.
 *
 * Typed commands never reach this: Vite swaps each generated binding module
 * for its `src/dev/mock-ipc/<feature>.ts` twin (see `vite.config.ts`). What is
 * left is the `@tauri-apps/api` runtime itself: the `main` window, the event
 * bus (so typed and legacy events reach their listeners) and the legacy
 * `invoke` calls, answered by the legacy shim.
 */
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';

import { mockNotes } from './database';
import { MockMedia } from './mock-media';
import { snapshotUrl } from './screen';
import { handleLegacyInvoke } from './legacy';

/** What `@tauri-apps/plugin-os` reads synchronously: a Mac on Apple Silicon. */
const OS_PLUGIN_INTERNALS = {
  platform: 'macos',
  os_type: 'macos',
  family: 'unix',
  arch: 'aarch64',
  version: '15.0.0',
  eol: '\n',
  exe_extension: '',
};

/** Installs the mocks. Call once, before the app renders. */
export function installMockIpc(): void {
  mockWindows('main');
  Object.defineProperty(window, '__TAURI_OS_PLUGIN_INTERNALS__', { value: OS_PLUGIN_INTERNALS });
  mockIPC(handleLegacyInvoke, { shouldMockEvents: true });
  mockAssetProtocol();
}

/**
 * `convertFileSrc`: note images are data URLs, recordings are the fixtures'
 * audio and snapshots the fixture slides, served by Vite.
 */
function mockAssetProtocol(): void {
  const internals: unknown = Reflect.get(window, '__TAURI_INTERNALS__');
  if (typeof internals !== 'object' || internals === null) return;
  Reflect.set(
    internals,
    'convertFileSrc',
    (path: string) => MockMedia.url(path) ?? snapshotUrl(path) ?? mockNotes.imageUrl(path) ?? '',
  );
}
