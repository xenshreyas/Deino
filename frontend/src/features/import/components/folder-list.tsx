/** The watched folders: path, "Move to processed" and Remove for each. */
import { useId } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { Folder, ICON_PROPS } from '@/ui/icons';
import { SettingsCard } from '@/ui/settings-section';

import { importCopy as copy } from '../copy';
import { useRemoveWatchedFolder, useSetMoveToProcessed } from '../queries';
import type { WatchedFolder } from '../types';

/** Explains a change that did not save. */
function failed(error: Error) {
  toast.error(copy.changeFailed, { description: error.message });
}

/** One folder. */
function FolderRow({ folder }: { folder: WatchedFolder }) {
  const switchId = useId();
  const move = useSetMoveToProcessed();
  const remove = useRemoveWatchedFolder();
  return (
    <div className="flex flex-col gap-2 px-4 py-3">
      <div className="flex min-w-0 items-center gap-2">
        <Folder {...ICON_PROPS} className="shrink-0 text-fg-muted" />
        <span className="truncate font-mono text-xs text-fg" title={folder.path}>
          {folder.path}
        </span>
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto"
          aria-label={copy.removeLabel(folder.path)}
          disabled={remove.isPending}
          onClick={() => {
            remove.mutate(folder.id, { onError: failed });
          }}
        >
          {copy.remove}
        </Button>
      </div>
      <div className="flex items-center justify-between gap-4 pl-6">
        <label htmlFor={switchId} className="text-xs text-fg-muted">
          {copy.moveLabel} · {copy.moveHint}
        </label>
        <Switch
          id={switchId}
          checked={folder.moveToProcessed}
          onCheckedChange={(on) => {
            move.mutate({ id: folder.id, on }, { onError: failed });
          }}
        />
      </div>
    </div>
  );
}

/** Every folder, or the empty state. */
export function FolderList({ folders }: { folders: readonly WatchedFolder[] }) {
  if (folders.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border px-4 py-6 text-center">
        <p className="text-sm text-fg">{copy.noFoldersTitle}</p>
        <p className="mt-1 text-xs text-fg-muted">{copy.noFoldersHint}</p>
      </div>
    );
  }
  return (
    <SettingsCard label={copy.foldersTitle}>
      {folders.map((folder) => (
        <FolderRow key={folder.id} folder={folder} />
      ))}
    </SettingsCard>
  );
}
