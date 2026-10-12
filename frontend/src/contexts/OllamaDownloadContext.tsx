import React, { createContext, useContext, useMemo, useState } from 'react';
import { toast } from 'sonner';

import type { OllamaPull } from '@/features/llm';
import { useIpcEvent } from '@/lib/ipc/events';
import { events } from '@/lib/ipc/generated/llm';

/**
 * Ollama download state, kept for the app's lifetime so progress survives
 * the settings dialog closing and duplicate downloads can be refused.
 * Progress arrives as the `llm` plugin's `ollamaPull` events (FND-09).
 */

interface OllamaDownloadState {
  downloadProgress: Map<string, number>;  // modelName -> progress (0-100)
  downloadingModels: Set<string>;         // Set of model names currently downloading
}

interface OllamaDownloadContextType extends OllamaDownloadState {
  isDownloading: (modelName: string) => boolean;
  getProgress: (modelName: string) => number | undefined;
}

const OllamaDownloadContext = createContext<OllamaDownloadContextType | null>(null);

export const useOllamaDownload = () => {
  const context = useContext(OllamaDownloadContext);
  if (!context) {
    throw new Error('useOllamaDownload must be used within an OllamaDownloadProvider');
  }
  return context;
};

/** `map` with `model` set to `percent`, or removed when `percent` is `null`. */
function withProgress(map: Map<string, number>, model: string, percent: number | null) {
  const next = new Map(map);
  if (percent === null) next.delete(model);
  else next.set(model, percent);
  return next;
}

export function OllamaDownloadProvider({ children }: { children: React.ReactNode }) {
  const [downloadProgress, setDownloadProgress] = useState<Map<string, number>>(new Map());

  useIpcEvent(events.ollamaPull, ({ model, status }: OllamaPull) => {
    if (status.kind === 'progress') {
      setDownloadProgress((previous) => withProgress(previous, model, status.percent));
      return;
    }
    setDownloadProgress((previous) => withProgress(previous, model, null));
    // UX-31: a finished download is quiet (the list shows it); only a failure says so.
    if (status.kind !== 'done') {
      toast.error(`Download failed: ${model}`, { description: status.message, duration: 6000 });
    }
  });

  const downloadingModels = useMemo(() => new Set(downloadProgress.keys()), [downloadProgress]);
  const contextValue: OllamaDownloadContextType = {
    downloadProgress,
    downloadingModels,
    isDownloading: (modelName: string) => downloadingModels.has(modelName),
    getProgress: (modelName: string) => downloadProgress.get(modelName),
  };

  return (
    <OllamaDownloadContext.Provider value={contextValue}>
      {children}
    </OllamaDownloadContext.Provider>
  );
}
