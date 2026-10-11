/**
 * The Action Items page's UI state: the filter edits on screen, the bulk
 * selection, the filter menu and the delete confirmation. A small Zustand
 * store, because the page's keyboard shortcuts (`F`, ⌘A, Esc) are a window
 * listener that must read and change it outside React's render.
 *
 * ⌘A sets `allSelected` rather than a list of IDs: the page derives the
 * selection from the items on screen, so it never needs them here.
 */
import { create } from 'zustand';

import { EMPTY_SELECTION, type Selection } from './selection';
import type { ActionItemFilter, ActionItemId } from './types';

/** The page's state and its transitions. */
export interface PageState {
  /** The filter on screen when it differs from the tab's, else `null`. */
  draft: ActionItemFilter | null;
  selection: Selection;
  /** ⌘A: everything listed is selected until the next selection change. */
  allSelected: boolean;
  menuOpen: boolean;
  /** Items waiting for the delete confirmation. */
  deleting: readonly ActionItemId[];
  setDraft: (draft: ActionItemFilter | null) => void;
  setSelection: (selection: Selection) => void;
  selectAll: () => void;
  clearSelection: () => void;
  setMenuOpen: (open: boolean) => void;
  askDelete: (ids: readonly ActionItemId[]) => void;
  /** Back to a fresh page (another tab, or the page opened again). */
  reset: () => void;
}

const INITIAL = {
  draft: null,
  selection: EMPTY_SELECTION,
  allSelected: false,
  menuOpen: false,
  deleting: [],
} as const;

/** Builds a page store; tests create their own. */
export function createPageStore() {
  return create<PageState>()((set) => ({
    ...INITIAL,
    setDraft: (draft) => {
      set({ draft, selection: EMPTY_SELECTION, allSelected: false });
    },
    setSelection: (selection) => {
      set({ selection, allSelected: false });
    },
    selectAll: () => {
      set({ allSelected: true });
    },
    clearSelection: () => {
      set({ selection: EMPTY_SELECTION, allSelected: false });
    },
    setMenuOpen: (menuOpen) => {
      set({ menuOpen });
    },
    askDelete: (deleting) => {
      set({ deleting });
    },
    reset: () => {
      set(INITIAL);
    },
  }));
}

/** The page's store. */
export const usePageStore = createPageStore();
