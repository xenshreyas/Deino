/**
 * Which line is being edited inline (TRN-02): at most one in the app. The
 * Edit action and a double-click start it; the editor ends it. A bubble
 * reads only whether it is the one, so starting an edit re-renders two rows.
 */
import { create } from 'zustand';

import type { SegmentId } from '../types';

/** The editing state. */
interface EditingState {
  /** The line being edited, if any. */
  editing: SegmentId | null;
}

/** The line being edited. */
export const useEditingStore = create<EditingState>()(() => ({ editing: null }));

/** Starts editing `id` (ending any other edit). */
export function startEditing(id: SegmentId): void {
  useEditingStore.setState({ editing: id });
}

/** Ends editing `id`, if it is still the one being edited. */
export function stopEditing(id: SegmentId): void {
  if (useEditingStore.getState().editing === id) useEditingStore.setState({ editing: null });
}

/** Whether `id` is being edited. */
export function useIsEditing(id: SegmentId): boolean {
  return useEditingStore((state) => state.editing === id);
}
