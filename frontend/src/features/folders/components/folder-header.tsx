/**
 * The folder page's header: the icon (click to change), the name (click to
 * rename, Enter or blur saves, Escape cancels), where the folder sits, and
 * the actions: Integrations (AUT-01's shortcut, resting) and the ⋯ menu.
 */
import { type FormEvent, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { ICON_PROPS, Plug } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';
import { Hint } from '@/ui/tooltip';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import { useRenameFolder, useSetFolderIcon } from '../mutations';
import { folderPath } from '../tree';
import type { Folder, FolderTreeEntry } from '../types';
import { FolderMenu } from './folder-menu';
import { IconGrid } from './icon-grid';

/** The icon tile; opens the icon grid. */
function FolderIconButton({ folder }: { folder: Folder }) {
  const [open, setOpen] = useState(false);
  const setIcon = useSetFolderIcon();
  const FolderIcon = folderIcon(folder.icon);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Hint label={foldersCopy.changeIcon}>
        <PopoverTrigger
          aria-label={foldersCopy.changeIcon}
          className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-bg-muted text-fg-muted transition-colors duration-(--dur-fast) hover:text-fg"
        >
          <FolderIcon {...ICON_PROPS} />
        </PopoverTrigger>
      </Hint>
      <PopoverContent align="start" className="w-auto p-2">
        <IconGrid
          value={folder.icon}
          onChange={(icon) => {
            setIcon.mutate(
              { id: folder.id, icon },
              {
                onSuccess: () => {
                  setOpen(false);
                },
              },
            );
          }}
        />
      </PopoverContent>
    </Popover>
  );
}

/** The name field; mounted per edit so it starts from the current name. */
function NameField({ folder, onDone }: { folder: Folder; onDone: () => void }) {
  const [draft, setDraft] = useState(folder.name);
  const rename = useRenameFolder();
  const save = (event?: FormEvent) => {
    event?.preventDefault();
    const name = draft.trim();
    if (name === '' || name === folder.name) {
      onDone();
      return;
    }
    rename.mutate(
      { id: folder.id, name },
      {
        onSettled: onDone,
        onError: (error) => toast.error(foldersCopy.actionFailed, { description: error.message }),
      },
    );
  };
  return (
    <form onSubmit={save} className="min-w-0 flex-1">
      <input
        aria-label={foldersCopy.rename}
        value={draft}
        autoFocus
        onChange={(event) => {
          setDraft(event.target.value);
        }}
        onBlur={() => {
          save();
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') onDone();
        }}
        className="w-full bg-transparent text-xl font-semibold text-fg outline-none"
      />
    </form>
  );
}

/** The name, editable in place. */
function FolderName({ folder }: { folder: Folder }) {
  const [editing, setEditing] = useState(false);
  if (editing)
    return (
      <NameField
        folder={folder}
        onDone={() => {
          setEditing(false);
        }}
      />
    );
  return (
    <h1 className="min-w-0 text-xl font-semibold text-balance text-fg">
      <button
        type="button"
        title={foldersCopy.rename}
        onClick={() => {
          setEditing(true);
        }}
        className={cn(
          'cursor-text rounded-sm text-left decoration-bg-muted decoration-2 underline-offset-[6px]',
          'transition-[text-decoration-color] duration-(--dur-fast) hover:underline hover:decoration-border-strong',
        )}
      >
        {folder.name}
      </button>
    </h1>
  );
}

/** Integrations: a resting button until automations (AUT-01) fill it. */
function IntegrationsButton() {
  return (
    <Hint label={foldersCopy.integrationsSoon}>
      <span tabIndex={0} className="rounded-md">
        <Button variant="secondary" size="sm" disabled>
          <Plug {...ICON_PROPS} />
          {foldersCopy.integrations}
        </Button>
      </span>
    </Hint>
  );
}

/** Icon, name, path and actions. */
export function FolderHeader({
  folder,
  entries,
}: {
  folder: Folder;
  entries: readonly FolderTreeEntry[];
}) {
  const parents = folderPath(entries, folder.id).slice(0, -1);
  return (
    <header className="flex items-start gap-3 pb-2">
      <FolderIconButton folder={folder} />
      <div className="min-w-0 flex-1">
        <FolderName folder={folder} />
        {parents.length === 0 ? null : (
          <p className="mt-0.5 truncate text-sm text-fg-muted">
            {foldersCopy.inside(parents.join(' / '))}
          </p>
        )}
      </div>
      <div className="flex shrink-0 items-center gap-1 pt-0.5">
        <IntegrationsButton />
        <FolderMenu folder={folder} entries={entries} className="size-7" />
      </div>
    </header>
  );
}
