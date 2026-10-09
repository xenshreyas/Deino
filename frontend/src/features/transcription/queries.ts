/**
 * TanStack Query hooks for the model manager.
 *
 * Model files are not database rows, so `DbChanged` never covers them:
 * {@link useModelEvents} keeps the list fresh from Rust's `ModelChanged` and
 * `ModelDownloadProgress` events instead.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { events } from '@/lib/ipc/generated/transcription';
import { useIpcEvent } from '@/lib/ipc/events';

import { transcriptionApi } from './api';
import type { DownloadProgress, ModelFileView } from './types';

/** Query keys; every key starts with `transcriptionKeys.all`. */
export const transcriptionKeys = {
  all: ['transcription'] as const,
  models: () => [...transcriptionKeys.all, 'models'] as const,
};

/** Every Parakeet Unified file with its state on this Mac. */
export const modelsQuery = () =>
  queryOptions({
    queryKey: transcriptionKeys.models(),
    queryFn: transcriptionApi.listModels,
  });

/** Every Parakeet Unified file with its state on this Mac. */
export function useModels() {
  return useQuery(modelsQuery());
}

/** Writes a download's progress into the cached list. */
function withProgress(models: ModelFileView[] | undefined, progress: DownloadProgress) {
  return models?.map((model) =>
    model.id === progress.id
      ? {
          ...model,
          state: { kind: 'downloading' as const, downloadedBytes: progress.downloadedBytes },
        }
      : model,
  );
}

/** Keeps the cached model list in step with Rust's model events while mounted. */
export function useModelEvents(): void {
  const client = useQueryClient();
  useIpcEvent(events.modelChanged, () => {
    void client.invalidateQueries({ queryKey: transcriptionKeys.models() });
  });
  useIpcEvent(events.modelDownloadProgress, ({ progress }) => {
    client.setQueryData(transcriptionKeys.models(), (models: ModelFileView[] | undefined) =>
      withProgress(models, progress),
    );
  });
}

/** A mutation that refreshes the model list when it settles. */
function useModelMutation<T>(mutationFn: (id: ModelFileView['id']) => Promise<T>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSettled: () => client.invalidateQueries({ queryKey: transcriptionKeys.models() }),
  });
}

/** Downloads (or resumes) a file; settles when it is ready or stops. */
export function useDownloadModel() {
  return useModelMutation(transcriptionApi.downloadModel);
}

/** Stops a download, keeping what arrived. */
export function useCancelDownload() {
  return useModelMutation(transcriptionApi.cancelDownload);
}

/** Deletes a file. */
export function useDeleteModel() {
  return useModelMutation(transcriptionApi.deleteModel);
}
