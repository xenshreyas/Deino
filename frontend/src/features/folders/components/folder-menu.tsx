/**
 * A folder's ⋯ menu and the dialogs it opens: new subfolder, rename, icon,
 * move (the keyboard path for dragging), delete. The tree rows and the
 * folder page both use it; each instance owns its dialogs.
 */
import { useMatchRoute, useNavigate } from '@tanstack/react-router';
import { Fragment, useState } from 'react';

import { cn } from '@/ui/cn';
import {
  Ellipsis,
  FolderInput,
  FolderPlus,
  type Icon,
  ICON_PROPS,
  Palette,
  Pencil,
  Trash2,
} from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { foldersCopy } from '../copy';
import { subtreeIds } from '../tree';
import type { Folder, FolderTreeEntry } from '../types';
import { DeleteFolderDialog, FolderIconDialog, MoveFolderDialog } from './folder-dialogs';
import { FolderNameDialog, type FolderNameTarget } from './folder-name-dialog';

/** What a menu item opens. */
type Action = 'subfolder' | 'rename' | 'icon' | 'move' | 'delete';

/** The menu's items, in order; the destructive one comes last. */
const ITEMS: readonly { action: Action; label: string; icon: Icon }[] = [
  { action: 'subfolder', label: foldersCopy.newSubfolder, icon: FolderPlus },
  { action: 'rename', label: foldersCopy.rename, icon: Pencil },
  { action: 'icon', label: foldersCopy.changeIcon, icon: Palette },
  { action: 'move', label: foldersCopy.moveTo, icon: FolderInput },
  { action: 'delete', label: foldersCopy.delete, icon: Trash2 },
];

/** Props of {@link FolderMenu}. */
export interface FolderMenuProps {
  folder: Folder;
  entries: readonly FolderTreeEntry[];
  className?: string | undefined;
}

/** Leaves the folder page when the folder (or one above it) is deleted. */
function useLeaveDeleted(folder: Folder, entries: readonly FolderTreeEntry[]) {
  const matchRoute = useMatchRoute();
  const navigate = useNavigate();
  return () => {
    const match = matchRoute({ to: '/folders/$folderId' });
    if (match === false) return;
    const doomed = [...subtreeIds(entries, folder.id)];
    if (doomed.some((id) => id === match.folderId)) void navigate({ to: '/folders' });
  };
}

/** The ⋯ trigger and its menu. */
function ActionsMenu({
  folder,
  className,
  onAction,
}: Omit<FolderMenuProps, 'entries'> & { onAction: (action: Action) => void }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={foldersCopy.rowActions(folder.name)}
        className={cn(
          'flex size-6 items-center justify-center rounded-sm text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg hover:text-fg',
          className,
        )}
      >
        <Ellipsis {...ICON_PROPS} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {ITEMS.map(({ action, label, icon: ItemIcon }) => (
          <Fragment key={action}>
            {action === 'delete' ? <DropdownMenuSeparator /> : null}
            <DropdownMenuItem
              destructive={action === 'delete'}
              onSelect={() => {
                onAction(action);
              }}
            >
              <ItemIcon {...ICON_PROPS} />
              {label}
            </DropdownMenuItem>
          </Fragment>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** What the name dialog does for the open action, if it is a naming one. */
function nameTarget(open: Action | null, folder: Folder): FolderNameTarget {
  if (open === 'subfolder') return { kind: 'create', parent: folder };
  if (open === 'rename') return { kind: 'rename', folder };
  return null;
}

/** The ⋯ menu and its dialogs. */
export function FolderMenu({ folder, entries, className }: FolderMenuProps) {
  const [open, setOpen] = useState<Action | null>(null);
  const leaveDeleted = useLeaveDeleted(folder, entries);
  const close = () => {
    setOpen(null);
  };
  const naming = nameTarget(open, folder);
  const when = (action: Action) => (open === action ? folder : null);
  return (
    <>
      <ActionsMenu folder={folder} className={className} onAction={setOpen} />
      <FolderNameDialog target={naming} onClose={close} />
      <FolderIconDialog folder={when('icon')} onClose={close} />
      <MoveFolderDialog folder={when('move')} entries={entries} onClose={close} />
      <DeleteFolderDialog folder={when('delete')} onClose={close} onDeleted={leaveDeleted} />
    </>
  );
}
