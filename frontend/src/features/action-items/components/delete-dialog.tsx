/**
 * Confirms deleting action items: deletion cannot be undone, so it is one
 * of the few actions behind a dialog (ui.md §3 Dialog, "Confirmations").
 */
import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';

import { copy } from '../copy';

/** Props of {@link DeleteDialog}. */
export interface DeleteDialogProps {
  /** How many items would be deleted; 0 keeps the dialog closed. */
  count: number;
  onConfirm: () => void;
  onClose: () => void;
}

/** "Delete 3 action items?" with Cancel and Delete. */
export function DeleteDialog({ count, onConfirm, onClose }: DeleteDialogProps) {
  return (
    <Dialog
      open={count > 0}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent hideClose>
        <DialogHeader>
          <DialogTitle>{copy.deleteDialog.title(count)}</DialogTitle>
          <DialogDescription>{copy.deleteDialog.body}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {copy.deleteDialog.cancel}
          </Button>
          <Button
            variant="danger-solid"
            // The dialog exists to confirm this one action.
            autoFocus
            onClick={() => {
              onConfirm();
              onClose();
            }}
          >
            {copy.deleteDialog.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
