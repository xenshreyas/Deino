/**
 * Typed calls to the `importers` plugin (CAP-12). Each resolves to the data
 * or throws an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/importers';
import { unwrap } from '@/lib/ipc/result';

import type {
  ImportPreview,
  ImportProgress,
  ImportSource,
  ImportSourceStatus,
  ImportTarget,
} from './types';

export const importersApi = {
  /** Every source and whether its key is saved; never the key. */
  sources: (): Promise<ImportSourceStatus[]> => unwrap(commands.sources()),
  /** Saves a source's key in the Keychain and switches its service on. */
  saveKey: (source: ImportSource, key: string): Promise<null> =>
    unwrap(commands.saveKey(source, key)),
  /** Forgets a source's key and switches its service off. */
  forgetKey: (source: ImportSource): Promise<null> => unwrap(commands.forgetKey(source)),
  /** Opens the folder dialog; `null` when cancelled. */
  pickFolder: (): Promise<string | null> => unwrap(commands.pickFolder()),
  /** Counts what an import would add. */
  preview: (target: ImportTarget): Promise<ImportPreview> => unwrap(commands.preview(target)),
  /** Starts the import in the background. */
  start: (target: ImportTarget): Promise<ImportProgress> => unwrap(commands.start(target)),
  /** Stops the running import after its current meeting. */
  cancel: (): Promise<boolean> => unwrap(commands.cancel()),
  /** The latest progress. */
  status: (): Promise<ImportProgress> => unwrap(commands.status()),
};
