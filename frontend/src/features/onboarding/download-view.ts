/**
 * A local model's download as the AI step shows it. Both models onboarding
 * downloads (Parakeet and the embedding model) report the same
 * `ModelFileView`, so one pure rule covers them.
 */
import type { ModelFileView } from '@/features/transcription';

/** What a model row shows. */
export type DownloadView =
  /** The model list has not loaded yet. */
  | { kind: 'checking' }
  /** Not on disk (or only partly): offer the download. */
  | { kind: 'missing' }
  /** Arriving now. */
  | { kind: 'downloading'; percent: number }
  /** On disk and verified. */
  | { kind: 'ready' };

/** Whole percent of `bytes` out of `total`, within 0–100. */
export function percentOf(bytes: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(100, Math.max(0, Math.floor((bytes / total) * 100)));
}

/**
 * The row for `model` (`undefined` while loading, `null` when there is
 * none), preferring `liveBytes` from progress events over the last listing.
 * A download the UI started (`pending`) shows as one before the listing
 * catches up.
 */
export function downloadView(
  model: ModelFileView | null | undefined,
  liveBytes: number | null = null,
  pending = false,
): DownloadView {
  if (model === undefined) return { kind: 'checking' };
  if (model === null) return { kind: 'missing' };
  const { state } = model;
  switch (state.kind) {
    case 'ready':
      return { kind: 'ready' };
    case 'downloading': {
      const bytes = Math.max(liveBytes ?? 0, state.downloadedBytes);
      return { kind: 'downloading', percent: percentOf(bytes, model.sizeBytes) };
    }
    case 'missing':
    case 'partial':
      return pending
        ? { kind: 'downloading', percent: percentOf(liveBytes ?? 0, model.sizeBytes) }
        : { kind: 'missing' };
  }
}

/** Megabytes, rounded, for "Parakeet · 680 MB". */
export function sizeLabel(bytes: number): string {
  return `${String(Math.round(bytes / 1_000_000))} MB`;
}
