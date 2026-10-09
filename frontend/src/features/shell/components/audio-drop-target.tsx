import { listen } from '@tauri-apps/api/event';
import { useState } from 'react';
import { toast } from 'sonner';

import { ImportDropOverlay } from '@/components/ImportAudio';
import { getAudioFormatsDisplayList } from '@/constants/audioFormats';
import { useMountEffect } from '@/hooks/useMountEffect';
import { loadBetaFeatures } from '@/types/betaFeatures';

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

/** Reads the beta flag from storage: this sits above the ConfigProvider state. */
function importEnabled(): boolean {
  return loadBetaFeatures().importAndRetranscribe;
}

/** Turns a drop into an import, or tells the user why it was ignored. */
function handleDrop(paths: readonly string[], onAudioFile: (path: string) => void): void {
  const outcome = classifyAudioDrop(paths, importEnabled());
  if (outcome.kind === 'audio') onAudioFile(outcome.path);
  if (outcome.kind === 'import-disabled') {
    toast.error(shellCopy.importDisabledTitle, { description: shellCopy.importDisabledDescription });
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

  useMountEffect(() =>
    holdSubscriptions([
      listen('tauri://drag-enter', () => setOverlayVisible(importEnabled())),
      listen('tauri://drag-leave', () => setOverlayVisible(false)),
      listen<DragDropPayload>('tauri://drag-drop', (event) => {
        setOverlayVisible(false);
        handleDrop(event.payload.paths, onAudioFile);
      }),
    ]),
  );

  return <ImportDropOverlay visible={overlayVisible} />;
}
