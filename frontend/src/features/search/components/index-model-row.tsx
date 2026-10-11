/**
 * The embedding model's row on the Search index page: name and size, where
 * its download stands, and the matching action, with a 2px progress bar
 * while it downloads (ui.md §3 Toast: progress in `--accent`).
 */
import { toast } from 'sonner';

import { IpcError } from '@/lib/ipc';
import { Button } from '@/ui/button';

import { searchCopy } from '../copy';
import { useCancelDownload, useDeleteModel, useDownloadModel } from '../queries';
import type { ModelFileView } from '../types';

const copy = searchCopy.index;
const BYTES_PER_MEGABYTE = 1024 * 1024;

/** Props of {@link ModelRow}. */
export interface ModelRowProps {
  model: ModelFileView;
  /** Bytes from the latest progress event, fresher than `model.state`. */
  downloadedBytes: number | null;
}

/** How far the download is, 0–100. */
export function downloadPercent(model: ModelFileView, latest: number | null): number {
  const state = model.state;
  const known =
    state.kind === 'downloading' || state.kind === 'partial' ? state.downloadedBytes : 0;
  const bytes = state.kind === 'downloading' ? Math.max(known, latest ?? 0) : known;
  return Math.min(100, Math.floor((bytes / model.sizeBytes) * 100));
}

/** The status line under the model's name. */
export function modelStatus(model: ModelFileView, percent: number): string {
  switch (model.state.kind) {
    case 'ready':
      return copy.ready;
    case 'downloading':
      return copy.downloading(percent);
    case 'partial':
      return copy.paused(percent);
    case 'missing':
      return copy.missing;
  }
}

/** The model, its state and its action. */
export function ModelRow({ model, downloadedBytes }: ModelRowProps) {
  const percent = downloadPercent(model, downloadedBytes);
  const megabytes = Math.round(model.sizeBytes / BYTES_PER_MEGABYTE);
  return (
    <div className="relative flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm text-fg">{copy.modelName(model.name, megabytes)}</span>
        <span className="text-xs text-fg-muted tabular-nums">{modelStatus(model, percent)}</span>
      </div>
      <ModelAction model={model} />
      {model.state.kind === 'downloading' ? (
        <span
          role="progressbar"
          aria-valuenow={percent}
          aria-valuemin={0}
          aria-valuemax={100}
          className="absolute inset-x-0 bottom-0 h-0.5 origin-left bg-accent transition-transform duration-(--dur-slow) ease-out"
          style={{ transform: `scaleX(${String(percent / 100)})` }}
        />
      ) : null}
    </div>
  );
}

/** Shows a failed action; a cancelled download is the user's own doing. */
function onError(error: Error) {
  if (error instanceof IpcError && error.code === 'cancelled') return;
  toast.error(error.message);
}

/** The one action a model row offers in its state. */
function useModelAction(model: ModelFileView): {
  label: string;
  primary: boolean;
  run: () => void;
} {
  const download = useDownloadModel();
  const cancel = useCancelDownload();
  const remove = useDeleteModel();
  switch (model.state.kind) {
    case 'ready':
      return {
        label: copy.delete,
        primary: false,
        run: () => {
          remove.mutate(undefined, { onError });
        },
      };
    case 'downloading':
      return {
        label: copy.cancel,
        primary: false,
        run: () => {
          cancel.mutate(undefined, { onError });
        },
      };
    case 'partial':
    case 'missing': {
      const label = model.state.kind === 'partial' ? copy.resume : copy.download;
      return {
        label,
        primary: true,
        run: () => {
          download.mutate(undefined, { onError });
        },
      };
    }
  }
}

function ModelAction({ model }: { model: ModelFileView }) {
  const action = useModelAction(model);
  return (
    <Button size="sm" variant={action.primary ? 'primary' : 'secondary'} onClick={action.run}>
      {action.label}
    </Button>
  );
}
