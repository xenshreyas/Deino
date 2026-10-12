/**
 * The two models onboarding downloads, through the features that own them:
 * Parakeet Unified (transcription) and the embedding model (search).
 */
import { useState } from 'react';

import {
  useDownloadEmbeddingModel,
  useIndexStatus,
  useModelDownloadProgress,
} from '@/features/search';
import { usePatchSettings } from '@/features/settings';
import { activeModel, useDownloadModel, useModelEvents, useModels } from '@/features/transcription';

import { type DownloadView, downloadView } from '../download-view';

/** One local model: what its row shows and how to fetch it. */
export interface LocalModel {
  view: DownloadView;
  /** The file's size in bytes, once known. */
  sizeBytes: number | null;
  /** Starts (or resumes) the download. */
  download: () => void;
  /** Why the last download failed, if it did. */
  error: string | null;
}

/**
 * Parakeet Unified at the quant recommended for this Mac. A finished
 * download also makes it the transcription engine, so the "Try it" step
 * records with the model just fetched (whatever engine was saved before).
 */
export function useTranscriptionModel(): LocalModel {
  useModelEvents();
  const models = useModels();
  const download = useDownloadModel();
  const { mutate: patchSettings } = usePatchSettings();
  const model =
    models.data === undefined ? undefined : (activeModel(models.data, undefined) ?? null);
  return {
    view: downloadView(model, null, download.isPending),
    sizeBytes: model?.sizeBytes ?? null,
    download: () => {
      if (!model) return;
      download.mutate(model.id, {
        onSuccess: () => {
          patchSettings({ transcription: { engine: 'parakeet_unified', model: model.id } });
        },
      });
    },
    error: download.error?.message ?? null,
  };
}

/** The embedding model behind semantic search (AI-01). */
export function useEmbeddingModel(): LocalModel {
  const status = useIndexStatus();
  const download = useDownloadEmbeddingModel();
  const [liveBytes, setLiveBytes] = useState<number | null>(null);
  useModelDownloadProgress(setLiveBytes);
  const model = status.data?.model;
  return {
    view: downloadView(model, liveBytes, download.isPending),
    sizeBytes: model?.sizeBytes ?? null,
    download: () => {
      download.mutate();
    },
    error: download.error?.message ?? null,
  };
}
