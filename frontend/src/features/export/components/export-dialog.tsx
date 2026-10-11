import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { FileText, ICON_PROPS } from '@/ui/icons';

import { exportCopy, formatCopy } from '../copy';
import { messageOf } from '../hooks/use-share-actions';
import { useExportPreview, useSaveExport } from '../queries';
import type { ExportFormat, ExportSections, MeetingId } from '../types';
import { ExportPreview } from './export-preview';
import { FormatList } from './format-list';
import { SectionToggles } from './section-toggles';

/** What a new export includes: everything a reader needs, not the transcript. */
export const DEFAULT_SECTIONS: ExportSections = {
  attendees: true,
  notes: true,
  myNotes: true,
  actionItems: true,
  insights: true,
  transcript: false,
};

/** Formats whose content is fixed: JSON is everything, subtitles the transcript. */
const FIXED: Partial<Record<ExportFormat, (name: string) => string>> = {
  json: exportCopy.fixedContent,
  srt: exportCopy.transcriptOnly,
  vtt: exportCopy.transcriptOnly,
};

/** Props of {@link ExportDialog}. */
export interface ExportDialogProps {
  meetingId: MeetingId;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Export a meeting: format on the left, what to include and a live preview on the right. */
export function ExportDialog({ meetingId, open, onOpenChange }: ExportDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[780px] gap-0 p-0">
        {open ? (
          <ExportDialogBody
            meetingId={meetingId}
            onDone={() => {
              onOpenChange(false);
            }}
          />
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

/** The dialog's choices, its preview and the save that closes it. */
function useExportDialog(meetingId: MeetingId, onDone: () => void) {
  const [format, setFormat] = useState<ExportFormat>('markdown');
  const [sections, setSections] = useState<ExportSections>(DEFAULT_SECTIONS);
  const preview = useExportPreview(meetingId, format, sections);
  const save = useSaveExport(meetingId);
  const exportFile = () => {
    save.mutate(
      { format, sections },
      {
        onSuccess: (saved) => {
          if (saved === null) return;
          toast.success(exportCopy.saved(saved.fileName));
          onDone();
        },
        onError: (error) => toast.error(messageOf(error)),
      },
    );
  };
  return { format, setFormat, sections, setSections, preview, saving: save.isPending, exportFile };
}

/** The dialog's content, mounted per opening so every export starts fresh. */
function ExportDialogBody({ meetingId, onDone }: { meetingId: MeetingId; onDone: () => void }) {
  const dialog = useExportDialog(meetingId, onDone);
  const fixed = FIXED[dialog.format];
  return (
    <>
      <DialogHeader className="px-6 pt-6 pb-5">
        <DialogTitle>{exportCopy.dialogTitle}</DialogTitle>
        <DialogDescription>{exportCopy.dialogDescription}</DialogDescription>
      </DialogHeader>
      <div className="grid h-[440px] grid-cols-[224px_1fr] gap-5 px-6">
        <FormatList value={dialog.format} onChange={dialog.setFormat} />
        <div className="flex min-h-0 flex-col gap-3">
          <SectionToggles
            value={dialog.sections}
            onChange={dialog.setSections}
            disabled={fixed !== undefined}
          />
          {fixed === undefined ? null : (
            <p className="-mt-1 text-xs text-fg-muted">{fixed(formatCopy[dialog.format].label)}</p>
          )}
          <ExportPreview
            preview={dialog.preview.data}
            stale={dialog.preview.isPlaceholderData}
            failed={dialog.preview.isError}
          />
        </div>
      </div>
      <ExportFooter
        fileName={dialog.preview.data?.fileName}
        saving={dialog.saving}
        onCancel={onDone}
        onExport={dialog.exportFile}
      />
    </>
  );
}

/** Props of {@link ExportFooter}. */
interface ExportFooterProps {
  fileName: string | undefined;
  saving: boolean;
  onCancel: () => void;
  onExport: () => void;
}

/** The file the save dialog will suggest, then Cancel and Export. */
function ExportFooter({ fileName, saving, onCancel, onExport }: ExportFooterProps) {
  return (
    <DialogFooter className="mt-5 items-center border-t border-border px-6 py-4">
      <span className="mr-auto flex min-w-0 items-center gap-1.5 text-xs text-fg-muted">
        <FileText {...ICON_PROPS} className="shrink-0" />
        <span className="truncate">{fileName}</span>
      </span>
      <Button variant="ghost" onClick={onCancel}>
        {exportCopy.cancel}
      </Button>
      <Button onClick={onExport} disabled={saving}>
        {saving ? exportCopy.exporting : exportCopy.confirm}
      </Button>
    </DialogFooter>
  );
}
