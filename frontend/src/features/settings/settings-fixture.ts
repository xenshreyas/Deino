/**
 * The default settings as Rust returns them, for tests: `Settings::default()`
 * as the bindings test writes it (`generated/settings-defaults.ts`), so a new
 * section's defaults arrive here without anyone copying them (REG-02).
 */
import { DEFAULT_SETTINGS } from '@/lib/ipc/generated/settings-defaults';

import type { Settings } from './types';

/** A fresh copy of the default settings. */
export function defaultSettings(): Settings {
  return structuredClone(DEFAULT_SETTINGS);
}
