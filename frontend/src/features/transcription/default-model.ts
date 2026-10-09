/**
 * The default engine's model for code outside React (the legacy onboarding
 * flow, which FND-11 rewrites): the Parakeet Unified file recommended for
 * this Mac's memory, its readiness, its download and its progress.
 */
import type { UnlistenFn } from '@tauri-apps/api/event';

import { transcriptionApi } from './api';
import { activeModel, isReady, megabytes } from './models';
import type { ModelFileView } from './types';

/** A download's progress in the units onboarding shows. */
export interface DefaultModelProgress {
  percent: number;
  downloadedMb: number;
  totalMb: number;
}

/** The recommended file for this Mac. */
export async function defaultModel(): Promise<ModelFileView | undefined> {
  return activeModel(await transcriptionApi.listModels(), undefined);
}

/** Whether the recommended file is downloaded and verified. */
export async function isDefaultModelReady(): Promise<boolean> {
  return isReady(await defaultModel());
}

/** Whether any Parakeet Unified file is downloading now. */
export async function isAnyModelDownloading(): Promise<boolean> {
  const models = await transcriptionApi.listModels();
  return models.some((model) => model.state.kind === 'downloading');
}

/** Downloads (or resumes) the recommended file; resolves once it is verified. */
export async function downloadDefaultModel(): Promise<void> {
  const model = await defaultModel();
  if (model === undefined || isReady(model)) return;
  await transcriptionApi.downloadModel(model.id);
}

/** Calls `onProgress` as a Parakeet Unified download progresses. */
export function onDefaultModelProgress(
  onProgress: (progress: DefaultModelProgress) => void,
): Promise<UnlistenFn> {
  return transcriptionApi.onDownloadProgress(({ downloadedBytes, totalBytes }) => {
    onProgress({
      percent: totalBytes > 0 ? Math.floor((downloadedBytes / totalBytes) * 100) : 0,
      downloadedMb: megabytes(downloadedBytes),
      totalMb: megabytes(totalBytes),
    });
  });
}
