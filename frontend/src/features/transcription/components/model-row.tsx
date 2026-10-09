import { Button } from '@/ui/button';

import { copy } from '../copy';
import { downloadedBytes, megabytes } from '../models';
import type { ModelFileView } from '../types';

/** What the user can do with one model file. */
export interface ModelRowActions {
  onDownload: () => void;
  onCancel: () => void;
  onDelete: () => void;
  onUse: () => void;
}

interface ModelRowProps {
  model: ModelFileView;
  inUse: boolean;
  actions: ModelRowActions;
}

/** One quant of the model: its size, its state and the matching actions. */
export function ModelRow({ model, inUse, actions }: ModelRowProps) {
  return (
    <li className="flex items-center justify-between gap-4 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm font-semibold">
          {model.quant}
          {model.recommended ? (
            <span className="ml-2 text-xs font-normal text-fg-muted">{copy.recommended}</span>
          ) : null}
        </span>
        <span className="text-xs text-fg-muted">
          {copy.size(megabytes(model.sizeBytes))} · {stateLabel(model)}
        </span>
      </div>
      <div className="flex shrink-0 gap-2">
        <RowButtons model={model} inUse={inUse} actions={actions} />
      </div>
    </li>
  );
}

function RowButtons({ model, inUse, actions }: ModelRowProps) {
  switch (model.state.kind) {
    case 'ready':
      return (
        <>
          <Button size="sm" variant="secondary" disabled={inUse} onClick={actions.onUse}>
            {inUse ? copy.inUse : copy.use}
          </Button>
          <Button size="sm" variant="danger" disabled={inUse} onClick={actions.onDelete}>
            {copy.delete}
          </Button>
        </>
      );
    case 'downloading':
      return (
        <Button size="sm" variant="secondary" onClick={actions.onCancel}>
          {copy.cancel}
        </Button>
      );
    case 'partial':
    case 'missing':
      return (
        <Button size="sm" onClick={actions.onDownload}>
          {model.state.kind === 'partial' ? copy.resume : copy.download}
        </Button>
      );
  }
}

/** "Downloaded", "Downloading… 42%", "Paused at 30%" or "Not downloaded". */
export function stateLabel(model: ModelFileView): string {
  const bytes = downloadedBytes(model);
  const percent = bytes === null ? 0 : Math.floor((bytes / model.sizeBytes) * 100);
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
