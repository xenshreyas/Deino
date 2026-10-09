/**
 * Arrow-key navigation of the folder tree (the WAI-ARIA tree pattern):
 * Up and Down move between visible folders, Right opens a closed folder or
 * steps into an open one, Left closes an open folder or steps out to its
 * parent, Home and End jump to the ends. Pure, so it is tested as a table.
 */
import type { FolderId, FolderTreeEntry } from './types';

/** What a key press does: move the focus, and/or open or close a folder. */
export interface TreeKeyEffect {
  focus?: FolderId;
  toggle?: FolderId;
}

/** The effect of `key` on the visible tree with `current` focused. */
export function treeKeyEffect(
  visible: readonly FolderTreeEntry[],
  tree: { collapsed: ReadonlySet<FolderId>; parents: ReadonlySet<FolderId> },
  current: FolderId,
  key: string,
): TreeKeyEffect | null {
  const index = visible.findIndex((entry) => entry.folder.id === current);
  const entry = visible[index];
  if (entry === undefined) return null;
  const at = (position: number): TreeKeyEffect | null => {
    const target = visible[position];
    return target === undefined ? null : { focus: target.folder.id };
  };
  const expandable = tree.parents.has(current);
  const closed = expandable && tree.collapsed.has(current);
  const open = expandable && !closed;
  switch (key) {
    case 'ArrowDown':
      return at(index + 1);
    case 'ArrowUp':
      return at(index - 1);
    case 'Home':
      return at(0);
    case 'End':
      return at(visible.length - 1);
    case 'ArrowRight':
      if (closed) return { toggle: current };
      return open ? at(index + 1) : null;
    case 'ArrowLeft':
      if (open) return { toggle: current };
      return entry.folder.parentId === null ? null : { focus: entry.folder.parentId };
    default:
      return null;
  }
}
