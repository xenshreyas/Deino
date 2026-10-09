import { type QueryClient, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useState } from 'react';
import { toast } from 'sonner';

import { ImportDropOverlay } from '@/components/ImportAudio';
import { getAudioFormatsDisplayList } from '@/constants/audioFormats';
import { settingsQuery } from '@/features/settings';
import { useMountEffect } from '@/hooks/useMountEffect';

import { classifyAudioDrop } from '../audio-drop';
import { shellCopy } from '../copy';
import { holdSubscriptions } from '../tauri-events';

/** Payload of Tauri's `tauri://drag-drop` window event. */
interface DragDropPayload {
  paths: string[];
}

interface AudioDropTargetProps {
  /** Called with the first dropped audio file. Captured on mount. */
  onAudioFile: (path: string) => void;
}

/**
 * Reads the import beta flag from the cached settings at drop time; the
 * listeners are attached once, so they must not capture a stale value.
 * Until the settings load, the feature's default (on) applies.
 */
function importEnabled(client: QueryClient): boolean {
  return client.getQueryData(settingsQuery().queryKey)?.beta.importAndRetranscribe ?? true;
}

/** Turns a drop into an import, or tells the user why it was ignored. */
function handleDrop(
  paths: readonly string[],
  enabled: boolean,
  onAudioFile: (path: string) => void,
): void {
  const outcome = classifyAudioDrop(paths, enabled);
  if (outcome.kind === 'audio') onAudioFile(outcome.path);
  if (outcome.kind === 'import-disabled') {
    toast.error(shellCopy.importDisabledTitle, {
      description: shellCopy.importDisabledDescription,
    });
  }
  if (outcome.kind === 'unsupported') {
    toast.error(shellCopy.unsupportedDropTitle, {
      description: shellCopy.supportedFormats(getAudioFormatsDisplayList()),
    });
  }
}

/**
 * Makes the whole window a drop target for audio files to import.
 *
 * Mount it only while the main app is shown: unmounting detaches the window
 * listeners, so drops during onboarding are ignored.
 */
export function AudioDropTarget({ onAudioFile }: AudioDropTargetProps) {
  const [overlayVisible, setOverlayVisible] = useState(false);
  const client = useQueryClient();

  useMountEffect(() =>
    holdSubscriptions([
      listen('tauri://drag-enter', () => {
        setOverlayVisible(importEnabled(client));
      }),
      listen('tauri://drag-leave', () => {
        setOverlayVisible(false);
      }),
      listen<DragDropPayload>('tauri://drag-drop', (event) => {
        setOverlayVisible(false);
        handleDrop(event.payload.paths, importEnabled(client), onAudioFile);
      }),
    ]),
  );

  return <ImportDropOverlay visible={overlayVisible} />;
}
