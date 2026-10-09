/// <reference types="vitest/config" />
/**
 * Vite build for the Deino UI, also used by Vitest.
 *
 * Plugin order matters: the TanStack Router plugin generates
 * `src/routeTree.gen.ts` and code-splits route files before the React plugin
 * transforms them. The Tauri block follows the Tauri 2 Vite guide and the
 * anarlog desktop app (MIT, see THIRD_PARTY_NOTICES.md).
 */
import { existsSync, readdirSync } from 'node:fs';
import { basename } from 'node:path';
import { fileURLToPath, URL } from 'node:url';

import tailwindcss from '@tailwindcss/vite';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import react from '@vitejs/plugin-react';
import { type Alias, defineConfig, searchForWorkspaceRoot } from 'vite';

/** Fixed dev-server port; `src-tauri/tauri.conf.json` `build.devUrl` points here. */
const DEV_PORT = 3118;
/** `pnpm dev:mock` port, apart from `tauri dev`'s so both can run at once. */
const MOCK_PORT = 3119;

const src = fileURLToPath(new URL('./src', import.meta.url));
const generatedDir = `${src}/lib/ipc/generated`;
const mockIpcDir = `${src}/dev/mock-ipc`;
/** The repository's `fixtures/`, which the mock IPC bundles. */
const fixturesDir = fileURLToPath(new URL('../fixtures', import.meta.url));

/**
 * `vite --mode mock` (`pnpm dev:mock`): every generated binding module
 * `@/lib/ipc/generated/<feature>` resolves to `src/dev/mock-ipc/<feature>.ts`,
 * which serves fixture data and `satisfies` the generated types. `shared.ts`
 * holds only types and constants, so it stays real. A generated module
 * without a mock fails here, so each feature card adds its mock.
 */
function mockIpcAliases(): Alias[] {
  const features = readdirSync(generatedDir)
    .map((file) => basename(file, '.ts'))
    .filter((feature) => feature !== 'shared');
  const missing = features.filter((feature) => !existsSync(`${mockIpcDir}/${feature}.ts`));
  if (missing.length > 0) {
    throw new Error(`dev:mock: no src/dev/mock-ipc/ mock for ${missing.join(', ')}`);
  }
  return [
    { find: /^@\/lib\/ipc\/generated\/(?!shared$)([\w-]+)$/, replacement: `${mockIpcDir}/$1.ts` },
  ];
}

/** Set by `tauri dev` when the webview runs on another device (mobile). */
const tauriDevHost = process.env.TAURI_DEV_HOST;

/** True for `tauri build --debug`: keep the bundle readable. */
const isTauriDebugBuild = Boolean(process.env.TAURI_ENV_DEBUG);

export default defineConfig(({ mode }) => ({
  plugins: [tanstackRouter({ target: 'react', autoCodeSplitting: true }), react(), tailwindcss()],
  resolve: {
    // BlockNote needs one ProseMirror instance. No alias or `dedupe` is
    // needed for that: the `pnpm.overrides` in package.json pin every
    // prosemirror-* package to a single version, so pnpm installs one copy.
    alias: [...(mode === 'mock' ? mockIpcAliases() : []), { find: '@', replacement: src }],
  },
  // Keep Rust compiler output visible in the `tauri dev` terminal.
  clearScreen: false,
  server: {
    port: mode === 'mock' ? MOCK_PORT : DEV_PORT,
    strictPort: true,
    // The mock IPC imports `fixtures/` from outside `frontend/`.
    fs: mode === 'mock' ? { allow: [searchForWorkspaceRoot(process.cwd()), fixturesDir] } : {},
    host: tauriDevHost ?? false,
    hmr: tauriDevHost ? { protocol: 'ws', host: tauriDevHost, port: DEV_PORT + 1 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    outDir: 'dist',
    // WKWebView on macOS 14.2+ (Safari 17) is the oldest engine we ship to.
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari17',
    minify: !isTauriDebugBuild,
    // Chunks load from disk inside the app, so size matters less than on the
    // web. The meeting route carries BlockNote (~930 kB) until NOT-01.
    chunkSizeWarningLimit: 1024,
    sourcemap: isTauriDebugBuild,
  },
  test: {
    environment: 'happy-dom',
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
    // Vitest stubs CSS; `styles/tokens.test.ts` reads the token sheet as text.
    css: { include: [/tokens\.css/] },
    // Legacy code logs a lot; show only console.error/warn output.
    onConsoleLog: (_log, type) => type === 'stderr',
  },
}));
