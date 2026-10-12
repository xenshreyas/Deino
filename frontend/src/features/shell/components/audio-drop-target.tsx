import { listen } from '@tauri-apps/api/event';
import { useState } from 'react';
import { toast } from 'sonner';

import { formatList, importCopy } from '@/features/import';
import { useMountEffect } from '@/hooks/useMountEffect';
import { ICON_PROPS_LARGE, Upload } from '@/ui/icons';

import { classifyAudioDrop } from '../audio-drop';
import { holdSubscriptions } from '../tauri-events';

/** Payload of Tauri's `tauri://drag-drop` window event. */
interface DragDropPayload {
  paths: string[];
}

interface AudioDropTargetProps {
  /** Called with the first dropped recording. Captured on mount. */
  onAudioFile: (path: string) => void;
}

/** Turns a drop into an import, or tells the user why it was ignored. */
function handleDrop(paths: readonly string[], onAudioFile: (path: string) => void): void {
  const outcome = classifyAudioDrop(paths);
  if (outcome.kind === 'audio') onAudioFile(outcome.path);
  if (outcome.kind === 'unsupported') {
    toast.error(importCopy.unsupportedDropTitle, {
      description: importCopy.supportedFormats(formatList()),
    });
  }
}

/** The calm overlay while a file is dragged over the window. */
function DropOverlay() {
  return (
    <div className="pointer-events-none fixed inset-0 z-(--z-dialog) flex items-center justify-center bg-bg/70 backdrop-blur-xs">
      <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed border-border-strong bg-surface-raised px-10 py-8 text-center shadow-dialog">
        <Upload {...ICON_PROPS_LARGE} className="text-fg-muted" />
        <p className="text-sm font-semibold text-fg">{importCopy.dropTitle}</p>
        <p className="max-w-[280px] text-xs text-fg-muted">{formatList()}</p>
      </div>
    </div>
  );
}

/**
 * Makes the whole window a drop target for recordings to import.
 *
 * Mount it only while the main app is shown: unmounting detaches the window
 * listeners, so drops during onboarding are ignored.
 */
export function AudioDropTarget({ onAudioFile }: AudioDropTargetProps) {
  const [overlayVisible, setOverlayVisible] = useState(false);

  useMountEffect(() =>
    holdSubscriptions([
      listen('tauri://drag-enter', () => {
        setOverlayVisible(true);
      }),
      listen('tauri://drag-leave', () => {
        setOverlayVisible(false);
      }),
      listen<DragDropPayload>('tauri://drag-drop', (event) => {
        setOverlayVisible(false);
        handleDrop(event.payload.paths, onAudioFile);
      }),
    ]),
  );

  return overlayVisible ? <DropOverlay /> : null;
}
