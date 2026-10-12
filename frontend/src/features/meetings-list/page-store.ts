/**
 * The Meetings page's UI state: the filter on screen (mirrored here for ⌘A),
 * the bulk selection, the filter menu and the merge confirmation. A small
 * Zustand store, because the page's keyboard shortcuts (`F`, ⌘A, Esc, ⌘⌫)
 * are a window listener that must read and change it outside React's render.
 */
import { create } from 'zustand';

import { EMPTY_SELECTION, type Selection } from './selection';
import { ALL_MEETINGS, type MeetingFilter, type MeetingId } from './types';

/** The page's state and its transitions. */
export interface PageState {
  /** The filter on screen; ⌘A selects every meeting it matches. */
  filter: MeetingFilter;
  selection: Selection;
  menuOpen: boolean;
  /** Meetings waiting for the merge confirmation. */
  merging: readonly MeetingId[];
  /** Shows `filter`; the selection, made under another filter, is cleared. */
  setFilter: (filter: MeetingFilter) => void;
  setSelection: (selection: Selection) => void;
  clearSelection: () => void;
  setMenuOpen: (open: boolean) => void;
  askMerge: (ids: readonly MeetingId[]) => void;
  /** Back to a fresh page showing `filter` (the page opened again). */
  reset: (filter: MeetingFilter) => void;
}

const INITIAL = {
  filter: ALL_MEETINGS,
  selection: EMPTY_SELECTION,
  menuOpen: false,
  merging: [],
} as const;

/** Builds a page store; tests create their own. */
export function createPageStore() {
  return create<PageState>()((set) => ({
    ...INITIAL,
    setFilter: (filter) => {
      set({ filter, selection: EMPTY_SELECTION });
    },
    setSelection: (selection) => {
      set({ selection });
    },
    clearSelection: () => {
      set({ selection: EMPTY_SELECTION });
    },
    setMenuOpen: (menuOpen) => {
      set({ menuOpen });
    },
    askMerge: (merging) => {
      set({ merging });
    },
    reset: (filter) => {
      set({ ...INITIAL, filter });
    },
  }));
}

/** The page's store. */
export const usePageStore = createPageStore();
