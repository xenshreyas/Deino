/**
 * Keeps the native window (title bar, traffic lights, vibrancy) in the same
 * theme as the page. `null` tells Tauri to follow the system.
 */
import { getCurrentWindow } from '@tauri-apps/api/window';

import type { ThemePreference } from './theme';

/** True inside the Tauri webview; false in a plain browser (`pnpm dev`). */
function insideTauri(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

/** Applies `preference` to the current Tauri window; a no-op in a browser. */
export async function syncWindowTheme(preference: ThemePreference): Promise<void> {
  if (!insideTauri()) return;
  try {
    await getCurrentWindow().setTheme(preference === 'system' ? null : preference);
  } catch (error: unknown) {
    console.error('[appearance] Could not set the window theme', error);
  }
}
