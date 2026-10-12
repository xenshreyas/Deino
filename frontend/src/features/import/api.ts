/**
 * Typed calls to the `import` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead. Rust
 * opens the dialogs; only a dropped file arrives here as a path.
 */
import { commands } from '@/lib/ipc/generated/import';
import { unwrap } from '@/lib/ipc/result';

import type {
  ImportCandidate,
  ImportedMeeting,
  WatchedFileId,
  WatchedFolder,
  WatchedFolderId,
  WatchFailure,
} from './types';

export const importApi = {
  /** Opens the file dialog; the chosen recording, or `null` when cancelled. */
  pickFile: (): Promise<ImportCandidate | null> => unwrap(commands.pickFile()),
  /** Checks a dropped file. */
  check: (path: string): Promise<ImportCandidate> => unwrap(commands.check(path)),
  /** Imports the file as a new meeting, which then transcribes in the background. */
  start: (path: string, title: string | null): Promise<ImportedMeeting> =>
    unwrap(commands.start(path, title)),
  /** Every watched folder, oldest first. */
  watchedFolders: (): Promise<WatchedFolder[]> => unwrap(commands.watchedFolders()),
  /** Opens the folder dialog and watches the folder; `null` when cancelled. */
  addWatchedFolder: (): Promise<WatchedFolder | null> => unwrap(commands.addWatchedFolder()),
  /** Stops watching a folder. */
  removeWatchedFolder: (id: WatchedFolderId): Promise<null> =>
    unwrap(commands.removeWatchedFolder(id)),
  /** Turns "Move to processed" on or off. */
  setMoveToProcessed: (id: WatchedFolderId, on: boolean): Promise<null> =>
    unwrap(commands.setMoveToProcessed(id, on)),
  /** Files a watched folder could not import, newest first. */
  failures: (): Promise<WatchFailure[]> => unwrap(commands.watchFailures()),
  /** Tries a failed file again. */
  retry: (id: WatchedFileId): Promise<null> => unwrap(commands.retryFailure(id)),
  /** Removes a failure from the list. */
  dismiss: (id: WatchedFileId): Promise<null> => unwrap(commands.dismissFailure(id)),
};
