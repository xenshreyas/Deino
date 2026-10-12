/**
 * The folder action dialogs: move (the keyboard path for dragging a folder),
 * change icon, and delete (which keeps every meeting).
 */
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { CommandItem } from '@/ui/command';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { ICON_PROPS, Inbox } from '@/ui/icons';

import { foldersCopy } from '../copy';
import { useDeleteFolder, useMoveFolder, useSetFolderIcon } from '../mutations';
import { canMoveFolder, moveTargets } from '../tree';
import type { Folder, FolderTreeEntry } from '../types';
import { FolderPicker } from './folder-picker';
import { IconGrid } from './icon-grid';

/** Props every folder dialog takes: the folder, or `null` while closed. */
export interface FolderDialogProps {
  folder: Folder | null;
  onClose: () => void;
}

function closeOn(onClose: () => void) {
  return (open: boolean) => {
    if (!open) onClose();
  };
}

function reportFailure(error: Error) {
  toast.error(foldersCopy.actionFailed, { description: error.message });
}

/** Moves `folder` under a parent (`null`: the top level), then closes. */
function useMoveTo(folder: Folder | null, onClose: () => void) {
  const move = useMoveFolder();
  const done = onClose;
  return (parentId: Folder['id'] | null) => {
    if (folder === null) return;
    move.mutate({ id: folder.id, parentId }, { onSuccess: done, onError: reportFailure });
  };
}

/** The folders `folder` may move into, with "Top level" first when it applies. */
function MoveTargets({
  folder,
  entries,
  onPick,
}: {
  folder: Folder;
  entries: readonly FolderTreeEntry[];
  onPick: (parentId: Folder['id'] | null) => void;
}) {
  const toTop = () => {
    onPick(null);
  };
  return (
    <FolderPicker
      entries={moveTargets(entries, folder.id)}
      onSelect={({ folder: target }) => {
        onPick(target.id);
      }}
    >
      {canMoveFolder(entries, folder.id, null) ? (
        <CommandItem value="top level" onSelect={toTop}>
          <Inbox {...ICON_PROPS} />
          {foldersCopy.topLevel}
        </CommandItem>
      ) : null}
    </FolderPicker>
  );
}

/** Moves a folder under another one, or to the top level. */
export function MoveFolderDialog({
  folder,
  entries,
  onClose,
}: FolderDialogProps & { entries: readonly FolderTreeEntry[] }) {
  const moveTo = useMoveTo(folder, onClose);
  return (
    <Dialog open={folder !== null} onOpenChange={closeOn(onClose)}>
      <DialogContent aria-describedby={undefined} className="gap-3 p-0 pt-6" hideClose>
        <DialogHeader className="px-6">
          <DialogTitle>{foldersCopy.moveTitle(folder?.name ?? '')}</DialogTitle>
        </DialogHeader>
        <div className="border-t border-border">
          {folder === null ? null : (
            <MoveTargets folder={folder} entries={entries} onPick={moveTo} />
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** Changes a folder's icon. */
export function FolderIconDialog({ folder, onClose }: FolderDialogProps) {
  const setIcon = useSetFolderIcon();
  return (
    <Dialog open={folder !== null} onOpenChange={closeOn(onClose)}>
      <DialogContent aria-describedby={undefined} className="max-w-[280px]">
        <DialogHeader>
          <DialogTitle>{foldersCopy.iconTitle}</DialogTitle>
        </DialogHeader>
        <IconGrid
          value={folder?.icon ?? null}
          onChange={(icon) => {
            if (folder === null) return;
            setIcon.mutate({ id: folder.id, icon }, { onSuccess: onClose, onError: reportFailure });
          }}
        />
      </DialogContent>
    </Dialog>
  );
}

/** Confirms and deletes a folder and its subfolders; meetings stay. */
export function DeleteFolderDialog({
  folder,
  onClose,
  onDeleted,
}: FolderDialogProps & { onDeleted: () => void }) {
  const remove = useDeleteFolder();
  const confirm = () => {
    if (folder === null) return;
    remove.mutate(folder.id, {
      onSuccess: () => {
        onClose();
        onDeleted();
      },
      onError: reportFailure,
    });
  };
  return (
    <Dialog open={folder !== null} onOpenChange={closeOn(onClose)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{foldersCopy.deleteTitle}</DialogTitle>
          <DialogDescription>{foldersCopy.deleteDescription(folder?.name ?? '')}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {foldersCopy.cancel}
          </Button>
          <Button variant="danger-solid" onClick={confirm} disabled={remove.isPending}>
            {foldersCopy.delete}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
