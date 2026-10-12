/**
 * A confirmation for an action that cannot be undone from the page: merging
 * meetings, or deleting them from the trash for good.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';

/** Props of {@link ConfirmDialog}. */
export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  /** A second paragraph, if any. */
  note?: string;
  confirm: string;
  cancel: string;
  /** The confirm button's look: `danger-solid` for deletions. */
  tone: 'primary' | 'danger-solid';
  /** Runs the action; the dialog closes when it settles. */
  onConfirm: () => Promise<unknown>;
  onClose: () => void;
}

/** The dialog; the confirm button waits for the action. */
export function ConfirmDialog(props: ConfirmDialogProps) {
  const [busy, setBusy] = useState(false);
  const run = () => {
    setBusy(true);
    void props.onConfirm().finally(() => {
      setBusy(false);
      props.onClose();
    });
  };
  return (
    <Dialog
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{props.title}</DialogTitle>
          <DialogDescription>{props.description}</DialogDescription>
          {props.note === undefined ? null : <p className="text-sm text-fg-muted">{props.note}</p>}
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={props.onClose}>
            {props.cancel}
          </Button>
          <Button variant={props.tone} onClick={run} disabled={busy}>
            {props.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
