import { useCallback, useState } from 'react';

/** Open state of the audio import dialog, shared by drag-drop and menus. */
export interface ImportDialogState {
  open: boolean;
  /** File chosen before the dialog opened (a drop), or `null` to pick one. */
  filePath: string | null;
  /** Opens the dialog, optionally with a file already chosen. */
  openWith: (filePath?: string | null) => void;
  /** Radix `onOpenChange`; closing forgets the chosen file. */
  onOpenChange: (open: boolean) => void;
}

/** Holds the import dialog's open state and preselected file. */
export function useImportDialogState(): ImportDialogState {
  const [open, setOpen] = useState(false);
  const [filePath, setFilePath] = useState<string | null>(null);

  const openWith = useCallback((path?: string | null) => {
    setFilePath(path ?? null);
    setOpen(true);
  }, []);

  const onOpenChange = useCallback((nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) setFilePath(null);
  }, []);

  return { open, filePath, openWith, onOpenChange };
}
