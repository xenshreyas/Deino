/** Typed calls to the `transcription` plugin. */
import { commands } from '@/lib/ipc/generated/transcription';
import { unwrap } from '@/lib/ipc/result';

import type { ModelFileId, ModelFileView } from './types';

/** The legacy transcript-config provider name of Parakeet Unified EN 0.6B. */
export const PARAKEET_UNIFIED_PROVIDER = 'parakeetUnified';

export const transcriptionApi = {
  /** Every Parakeet Unified file with its state on this Mac. */
  listModels: (): Promise<ModelFileView[]> => unwrap(commands.listModels()),
  /** Downloads (or resumes) and verifies a file; resolves when it is ready. */
  downloadModel: (id: ModelFileId): Promise<null> => unwrap(commands.downloadModel(id)),
  /** Stops a download, keeping what arrived. */
  cancelDownload: (id: ModelFileId): Promise<boolean> => unwrap(commands.cancelDownload(id)),
  /** Deletes a file. */
  deleteModel: (id: ModelFileId): Promise<null> => unwrap(commands.deleteModel(id)),
};
