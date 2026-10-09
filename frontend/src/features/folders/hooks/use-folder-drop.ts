/**
 * Makes an element a drop target for meetings and folders (ui.md §3 Nav
 * item: drag-and-drop target). A meeting dropped on a folder is filed
 * there; a folder dropped on a folder (or on the Folders heading, `null`)
 * moves under it, unless that would put it inside itself.
 */
import { type DragEvent, useState } from 'react';
import { toast } from 'sonner';

import { foldersCopy } from '../copy';
import { draggedKind, readDrag } from '../dnd';
import { useMoveFolder } from '../mutations';
import { canMoveFolder } from '../tree';
import type { Folder, FolderId, FolderTreeEntry } from '../types';
import { useFileMeeting } from './use-file-meeting';

/** The handlers to spread on the target, and whether a drag is over it. */
export interface FolderDrop {
  isOver: boolean;
  handlers: {
    onDragOver: (event: DragEvent) => void;
    onDragLeave: (event: DragEvent) => void;
    onDrop: (event: DragEvent) => void;
  };
}

/**
 * The drop effect for what is being dragged over `target`, or `null` when
 * it may not land there. A meeting is filed (a copy: it stays elsewhere
 * too); a folder moves. A folder's ID is hidden until the drop, which
 * refuses moves into its own subtree.
 */
function dropEffect(event: DragEvent, target: Folder | null): 'copy' | 'move' | null {
  const kind = draggedKind([...event.dataTransfer.types]);
  if (kind === 'meeting') return target === null ? null : 'copy';
  return kind === 'folder' ? 'move' : null;
}

/** Drop handling for the folder `target` (`null`: the top level). */
export function useFolderDrop(
  target: Folder | null,
  entries: readonly FolderTreeEntry[],
): FolderDrop {
  const [isOver, setIsOver] = useState(false);
  const { file } = useFileMeeting();
  const move = useMoveFolder();
  const targetId: FolderId | null = target?.id ?? null;
  const drop = (event: DragEvent) => {
    const payload = readDrag(event.dataTransfer);
    if (payload?.kind === 'meeting' && target !== null) file(target, payload.id);
    if (payload?.kind !== 'folder' || !canMoveFolder(entries, payload.id, targetId)) return;
    move.mutate(
      { id: payload.id, parentId: targetId },
      { onError: (error) => toast.error(foldersCopy.actionFailed, { description: error.message }) },
    );
  };
  return {
    isOver,
    handlers: {
      onDragOver: (event) => {
        const effect = dropEffect(event, target);
        if (effect === null) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = effect;
        setIsOver(true);
      },
      onDragLeave: () => {
        setIsOver(false);
      },
      onDrop: (event) => {
        event.preventDefault();
        setIsOver(false);
        drop(event);
      },
    },
  };
}
