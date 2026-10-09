/**
 * The sidebar tree's UI state: whether the Folders section is open and
 * which folders are collapsed. A small Zustand store because the tree, the
 * folder page and drops (which expand their target) all touch it. Not
 * persisted yet, like the rest of the shell layout (PARITY_PLAN §7.2).
 */
import { create } from 'zustand';

import type { FolderId } from './types';

/** The tree's open and collapsed state and its transitions. */
export interface FolderTreeState {
  open: boolean;
  collapsed: ReadonlySet<FolderId>;
  toggleOpen: () => void;
  toggleFolder: (id: FolderId) => void;
  expand: (id: FolderId) => void;
}

/** Builds a tree store; tests create their own. */
export function createFolderTreeStore() {
  return create<FolderTreeState>()((set) => ({
    open: true,
    collapsed: new Set(),
    toggleOpen: () => {
      set((state) => ({ open: !state.open }));
    },
    toggleFolder: (id) => {
      set((state) => {
        const collapsed = new Set(state.collapsed);
        if (!collapsed.delete(id)) collapsed.add(id);
        return { collapsed };
      });
    },
    expand: (id) => {
      set((state) => {
        if (!state.collapsed.has(id)) return state;
        const collapsed = new Set(state.collapsed);
        collapsed.delete(id);
        return { collapsed, open: true };
      });
    },
  }));
}

/** The app's folder tree state. */
export const useFolderTreeStore = createFolderTreeStore();
