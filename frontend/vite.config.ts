/// <reference types="vitest/config" />
/**
 * Vite build for the Deino UI, also used by Vitest.
 *
 * Plugin order matters: the TanStack Router plugin generates
 * `src/routeTree.gen.ts` and code-splits route files before the React plugin
 * transforms them. The Tauri block follows the Tauri 2 Vite guide and the
 * anarlog desktop app (MIT, see THIRD_PARTY_NOTICES.md).
 */
import { fileURLToPath, URL } from 'node:url';

import tailwindcss from '@tailwindcss/vite';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

/** Fixed dev-server port; `src-tauri/tauri.conf.json` `build.devUrl` points here. */
const DEV_PORT = 3118;

/** Set by `tauri dev` when the webview runs on another device (mobile). */
const tauriDevHost = process.env.TAURI_DEV_HOST;

/** True for `tauri build --debug`: keep the bundle readable. */
const isTauriDebugBuild = Boolean(process.env.TAURI_ENV_DEBUG);

export default defineConfig({
  plugins: [
    tanstackRouter({ target: 'react', autoCodeSplitting: true }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    // BlockNote needs one ProseMirror instance. No alias or `dedupe` is
    // needed for that: the `pnpm.overrides` in package.json pin every
    // prosemirror-* package to a single version, so pnpm installs one copy.
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  // Keep Rust compiler output visible in the `tauri dev` terminal.
  clearScreen: false,
  server: {
    port: DEV_PORT,
    strictPort: true,
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
    // Legacy code logs a lot; show only console.error/warn output.
    onConsoleLog: (_log, type) => type === 'stderr',
  },
});
