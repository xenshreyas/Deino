/**
 * Typed calls to the `settings` and `secrets` plugins. Each resolves to the
 * data or throws an `IpcError`; components use the hooks in `queries.ts`.
 *
 * There is deliberately no way to read a secret: the UI can save a key,
 * ask whether one is saved, and forget it.
 */
import { commands as secretCommands } from '@/lib/ipc/generated/secrets';
import { commands as settingsCommands } from '@/lib/ipc/generated/settings';
import { unwrap } from '@/lib/ipc/result';

import type { SecretName, SecretStatus, Settings, SettingsPatch } from './types';

export const settingsApi = {
  /** Every setting. */
  get: (): Promise<Settings> => unwrap(settingsCommands.get()),
  /** Applies a merge patch; rejects with `invalid`, changing nothing, for a bad one. */
  patch: (patch: SettingsPatch): Promise<Settings> => unwrap(settingsCommands.patch(patch)),
};

export const secretsApi = {
  /** Saves `value` as the key `name` in the Keychain; rejects a blank key with `invalid`. */
  set: (name: SecretName, value: string): Promise<null> => unwrap(secretCommands.set(name, value)),
  /** Whether the key is `missing`, `stored`, or `locked` (macOS refused access). */
  status: (name: SecretName): Promise<SecretStatus> => unwrap(secretCommands.status(name)),
  /** Forgets the key; succeeds when none is saved. */
  delete: (name: SecretName): Promise<null> => unwrap(secretCommands.delete(name)),
};
