/**
 * Pure rules over the model list: which file recordings use, and whether it
 * is ready. Shared by the model manager, recording start and onboarding.
 */
import type { ModelFileView } from './types';

/**
 * The file recordings will use: the selected one, or the recommended one
 * when nothing (or an unknown file) is selected.
 */
export function activeModel(
  models: readonly ModelFileView[],
  selected: string | undefined,
): ModelFileView | undefined {
  return models.find((model) => model.id === selected) ?? models.find((model) => model.recommended);
}

/** Whether `model` is downloaded and verified. */
export function isReady(model: ModelFileView | undefined): boolean {
  return model?.state.kind === 'ready';
}

/** Bytes on disk for a file that is downloading or paused, else `null`. */
export function downloadedBytes(model: ModelFileView): number | null {
  const { state } = model;
  return state.kind === 'downloading' || state.kind === 'partial' ? state.downloadedBytes : null;
}

/** Whole megabytes, for sizes in the model list. */
export function megabytes(bytes: number): number {
  return Math.round(bytes / 1_000_000);
}
