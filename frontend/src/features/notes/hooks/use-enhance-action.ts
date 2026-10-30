/**
 * The one way the UI writes notes: Enhance in the header (⌘⇧E), the empty
 * state, the regenerate banner and the template picker all call
 * {@link useEnhanceAction}. Rust refuses to replace a version the user
 * edited (`conflict`); the action then asks, through the confirm dialog the
 * header renders, and retries with consent. Once retention deleted the
 * transcript (PRV-01) the action is `blocked`: triggers disable themselves
 * and say why, and running it does nothing (Rust refuses too).
 */
import { create } from 'zustand';

import type { MeetingId } from '@/features/meetings';
import { useRegenerateBlock } from '@/features/retention';
import { IpcError } from '@/lib/ipc/result';

import { useEnhance } from '../queries';

/** Which meeting waits for the user's consent to replace edits. */
interface ConfirmState {
  pending: MeetingId | null;
  ask: (meeting: MeetingId) => void;
  close: () => void;
}

/** The confirm dialog's state, shared by every trigger and the dialog. */
export const useConfirmRegenerate = create<ConfirmState>()((set) => ({
  pending: null,
  ask: (pending) => {
    set({ pending });
  },
  close: () => {
    set({ pending: null });
  },
}));

/** True when `error` is Rust refusing to replace the user's edits. */
export function needsConsent(error: unknown): boolean {
  return error instanceof IpcError && error.code === 'conflict';
}

/** True when `error` is the run the user stopped: no failure, nothing to show. */
export function wasStopped(error: unknown): boolean {
  return error instanceof IpcError && error.code === 'cancelled';
}

/** What a trigger gets: run it, and whether notes are being written. */
export interface EnhanceAction {
  /** Writes notes; asks first when that would replace the user's edits. */
  run: () => void;
  /** Writes notes over the user's edits (after they agreed). */
  replace: () => void;
  isPending: boolean;
  /** Why notes cannot be written (the transcript was deleted), or `null`. */
  blocked: string | null;
  /** A failure worth showing (never the consent question). */
  error: Error | null;
  reset: () => void;
}

/** Writes notes for `meeting` (see the module docs). */
export function useEnhanceAction(meeting: MeetingId): EnhanceAction {
  const enhance = useEnhance(meeting);
  const ask = useConfirmRegenerate((state) => state.ask);
  const blocked = useRegenerateBlock(meeting);
  const failure =
    enhance.error !== null && !needsConsent(enhance.error) && !wasStopped(enhance.error)
      ? enhance.error
      : null;
  return {
    run: () => {
      if (blocked !== null) return;
      enhance.mutate(false, {
        onError: (error) => {
          if (needsConsent(error)) ask(meeting);
        },
      });
    },
    replace: () => {
      if (blocked === null) enhance.mutate(true);
    },
    isPending: enhance.isPending,
    blocked,
    error: failure,
    reset: enhance.reset,
  };
}
