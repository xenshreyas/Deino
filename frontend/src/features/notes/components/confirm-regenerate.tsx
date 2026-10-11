/**
 * "Replace your edits?": the one confirmation NOT-02 asks for (ui.md §3
 * Dialog, Granola §3.3). Opened by any Enhance trigger that Rust turned
 * away with `conflict`; the edited version stays in the history either way.
 */
import type { MeetingId } from '@/features/meetings';
import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';

import { notesCopy } from '../copy';
import { useConfirmRegenerate } from '../hooks/use-enhance-action';

/** Props of {@link ConfirmRegenerate}. */
export interface ConfirmRegenerateProps {
  meetingId: MeetingId;
  /** Writes new notes over the edits. */
  onConfirm: () => void;
}

/** The dialog, open while `meetingId` waits for consent. */
export function ConfirmRegenerate({ meetingId, onConfirm }: ConfirmRegenerateProps) {
  const open = useConfirmRegenerate((state) => state.pending === meetingId);
  const close = useConfirmRegenerate((state) => state.close);
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) close();
      }}
    >
      <DialogContent hideClose>
        <DialogHeader>
          <DialogTitle>{notesCopy.confirmTitle}</DialogTitle>
          <DialogDescription>{notesCopy.confirmBody}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={close}>
            {notesCopy.cancel}
          </Button>
          <Button
            autoFocus
            onClick={() => {
              close();
              onConfirm();
            }}
          >
            {notesCopy.confirmAction}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
