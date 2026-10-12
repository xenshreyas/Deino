/**
 * Processing details (UX-01, UX-05): every post-meeting stage with its
 * status and why it skipped or failed, a retry on a failed one. Opened from
 * the meeting's ⋯ menu; the header itself only speaks up on failure.
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
import { usePipelineProgress } from '../hooks/use-pipeline-progress';
import { usePipelineStatus, useRerunStage } from '../queries';
import type { MeetingId } from '../types';
import { StageItem } from './stage-item';

/** Props of {@link ProcessingDetailsDialog}. */
export interface ProcessingDetailsDialogProps {
  meetingId: MeetingId;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** The list of stages, live while the dialog is open. */
export function StageList({ meetingId }: { meetingId: MeetingId }) {
  const status = usePipelineStatus(meetingId);
  const rerun = useRerunStage(meetingId);
  usePipelineProgress(meetingId);
  if (status.isError) return <p className="text-sm text-fg-muted">{copy.loadFailed}</p>;
  const stages = status.data ?? [];
  if (status.isSuccess && stages.length === 0) {
    return <p className="text-sm text-fg-muted">{copy.detailsEmpty}</p>;
  }
  return (
    <div className="flex flex-col gap-2">
      <ul aria-label={copy.label} className="flex flex-col">
        {stages.map((stage) => (
          <StageItem
            key={stage.stage}
            stage={stage}
            retrying={rerun.isPending}
            onRetry={(id) => {
              rerun.mutate(id);
            }}
          />
        ))}
      </ul>
      {rerun.isError && (
        <p role="alert" className="text-xs text-danger">
          {rerun.error.message || copy.retryFailed}
        </p>
      )}
    </div>
  );
}

/** The dialog; its list only loads while it is open. */
export function ProcessingDetailsDialog({
  meetingId,
  open,
  onOpenChange,
}: ProcessingDetailsDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{copy.detailsTitle}</DialogTitle>
          <DialogDescription>{copy.detailsDescription}</DialogDescription>
        </DialogHeader>
        {open ? <StageList meetingId={meetingId} /> : null}
        <DialogFooter>
          <Button
            variant="secondary"
            onClick={() => {
              onOpenChange(false);
            }}
          >
            {copy.close}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
