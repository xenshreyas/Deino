/**
 * The "Edit with AI" bar's state (Zustand): whether it is open and which
 * text blocks it edits. The enhanced editor registers how to read its
 * selection, so the toolbar button and ⌘⇧E open the bar on what is selected.
 */
import { create } from 'zustand';

/** Reads the editor's selected text blocks (empty without a selection). */
export type SelectionReader = () => number[];

interface EditBarState {
  /** True while the bar is open. */
  open: boolean;
  /** The text blocks the edit is scoped to; empty for the whole notes. */
  scope: number[];
  /** The newest version's editor, while it is mounted. */
  reader: SelectionReader | null;
}

/** The bar's state. */
export const useEditBarStore = create<EditBarState>()(() => ({
  open: false,
  scope: [],
  reader: null,
}));

/** Opens the bar on the editor's selection (or the whole notes). */
export function openEditBar(): void {
  const { reader } = useEditBarStore.getState();
  useEditBarStore.setState({ open: true, scope: reader?.() ?? [] });
}

/** Closes the bar. */
export function closeEditBar(): void {
  useEditBarStore.setState({ open: false, scope: [] });
}

/** Edits the whole notes instead of the selection. */
export function clearEditScope(): void {
  useEditBarStore.setState({ scope: [] });
}

/** Registers the editor's selection reader; returns the unregister. */
export function registerSelectionReader(reader: SelectionReader): () => void {
  useEditBarStore.setState({ reader });
  return () => {
    if (useEditBarStore.getState().reader === reader) useEditBarStore.setState({ reader: null });
  };
}
