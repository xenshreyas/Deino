/**
 * "Resume recording" on a finished meeting (CAP-07, Granola §2.5): records
 * more of it, appended after what is there, and opens the record screen.
 * Shown while nothing records and the meeting is done processing (a
 * meeting still in the pipeline cannot take more audio yet).
 */
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';

import type { MeetingId } from '@/features/meetings';
import { usePipelineStatus } from '@/features/pipeline';
import { Button } from '@/ui/button';
import { ICON_PROPS, Mic } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { recordingApi } from '../api';
import { recordingCopy } from '../copy';
import { report, useRecordingState } from '../queries';

/** Whether the meeting's pipeline has finished (or never ran). */
function useFinished(meetingId: MeetingId): boolean {
  const { data: stages } = usePipelineStatus(meetingId);
  if (stages === undefined) return false;
  return stages.every((stage) => stage.status !== 'pending' && stage.status !== 'running');
}

/** The button, or nothing while it cannot be used. */
export function ResumeRecordingButton({ meetingId }: { meetingId: MeetingId }) {
  const navigate = useNavigate();
  const idle = useRecordingState().phase === 'idle';
  const finished = useFinished(meetingId);
  const resume = useMutation({
    mutationFn: () => recordingApi.resumeMeeting(meetingId),
    onSuccess: () => void navigate({ to: '/' }),
    onError: report(recordingCopy.resumeFailed),
  });
  if (!idle || !finished) return null;
  return (
    <Hint label={recordingCopy.resumeMeetingHint} side="bottom">
      <Button
        variant="ghost"
        size="sm"
        className="h-7 gap-1.5 px-2 text-xs text-fg-muted"
        disabled={resume.isPending}
        onClick={() => {
          resume.mutate();
        }}
      >
        <Mic {...ICON_PROPS} />
        {recordingCopy.resumeMeeting}
      </Button>
    </Hint>
  );
}
