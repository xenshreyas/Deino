/**
 * The edit's mutations: propose (nothing stored) and accept (a new version,
 * with an Undo toast that restores the previous version).
 */
import { useMutation } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { MeetingId } from '@/features/meetings';

import { enhanceApi } from '../api';
import { noteEditApi } from './api';
import { editCopy } from './copy';
import type { AcceptedEdit, EditProposal } from './types';

/** Asks for a proposal. */
export function useProposeEdit() {
  return useMutation({ mutationFn: noteEditApi.propose });
}

/** Restores the version an edit was made from: the edit's undo. */
export async function undoEdit(meetingId: MeetingId, accepted: AcceptedEdit): Promise<void> {
  await enhanceApi.restore(meetingId, accepted.previousVersion);
  toast(editCopy.undone);
}

/** Stores an accepted proposal; `DbChanged` shows the new version. */
export function useAcceptEdit(meetingId: MeetingId) {
  return useMutation({
    mutationFn: (proposal: EditProposal) =>
      noteEditApi.accept(meetingId, proposal.baseVersion, proposal.doc),
    onSuccess: (accepted) => {
      toast.success(editCopy.accepted, {
        action: {
          label: editCopy.undo,
          onClick: () => {
            undoEdit(meetingId, accepted).catch(() => {
              toast.error(editCopy.failed);
            });
          },
        },
      });
    },
  });
}
