/**
 * The import dialog (CAP-11): choose or drop a recording, name it, import.
 * Once the meeting exists it offers ORG-03's event chip, so the import can
 * be linked to the calendar event it was recorded for, and a way in.
 *
 * Steps follow the data, not stored state: a dropped file is checked by a
 * query, a picked one comes from the dialog's mutation, and the import
 * mutation's state says whether it is converting or done.
 */
import { useState } from 'react';

import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/ui/dialog';

import { importCopy as copy } from '../copy';
import { useCheckedFile, usePickFile, useStartImport } from '../queries';
import type { ImportCandidate } from '../types';
import { ChooseStep, DoneStep, ImportingStep, ReadyStep } from './import-steps';

/** Props of {@link ImportDialog}. */
export interface ImportDialogProps {
  open: boolean;
  /** A file dropped on the window, or `null` to choose one. */
  filePath: string | null;
  onOpenChange: (open: boolean) => void;
}

/** The dialog; its body remounts per file so every import starts fresh. */
export function ImportDialog({ open, filePath, onOpenChange }: ImportDialogProps) {
  const close = () => {
    onOpenChange(false);
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[480px]">
        <DialogHeader>
          <DialogTitle>{copy.dialogTitle}</DialogTitle>
          <DialogDescription>{copy.dialogDescription}</DialogDescription>
        </DialogHeader>
        {open ? <ImportFlow key={filePath ?? 'pick'} filePath={filePath} onClose={close} /> : null}
      </DialogContent>
    </Dialog>
  );
}

/** The current step of one import. */
function ImportFlow({ filePath, onClose }: { filePath: string | null; onClose: () => void }) {
  const [picked, setPicked] = useState<ImportCandidate | null>(null);
  const [title, setTitle] = useState('');
  const checked = useCheckedFile(picked === null ? filePath : null);
  const pick = usePickFile();
  const start = useStartImport();
  const candidate = picked ?? checked.data ?? null;
  const choose = () => {
    pick.mutate(undefined, {
      onSuccess: (file) => {
        if (file !== null) setPicked(file);
      },
    });
  };
  if (start.isSuccess) return <DoneStep meeting={start.data} onClose={onClose} />;
  if (start.isPending && candidate !== null) return <ImportingStep candidate={candidate} />;
  if (candidate === null) {
    const error = pick.error ?? checked.error;
    return <ChooseStep checking={checked.isLoading} error={error} onChoose={choose} />;
  }
  return (
    <ReadyStep
      candidate={candidate}
      title={title}
      error={start.error}
      onTitle={setTitle}
      onImport={() => {
        start.mutate({ path: candidate.path, title: title.trim() === '' ? null : title });
      }}
      onChooseAnother={choose}
    />
  );
}
