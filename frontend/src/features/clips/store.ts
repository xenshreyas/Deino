/**
 * The clip being named (Zustand): the transcript's Clip action and the
 * selection menu set it, and the one {@link ClipDialog} on the meeting page
 * shows while it is set.
 */
import { create } from 'zustand';

import type { ClipDraft } from './types';

interface ClipDraftState {
  draft: ClipDraft | null;
}

/** The pending clip, if any. */
export const useClipDraftStore = create<ClipDraftState>()(() => ({ draft: null }));

/** Opens the clip dialog for `draft`. */
export function startClip(draft: ClipDraft): void {
  useClipDraftStore.setState({ draft });
}

/** Closes the clip dialog. */
export function cancelClip(): void {
  useClipDraftStore.setState({ draft: null });
}
