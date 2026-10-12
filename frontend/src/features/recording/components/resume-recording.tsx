/**
 * "Resume recording" on a finished meeting (CAP-07, Granola §2.5): records
 * more of it, appended after what is there, and opens the record screen.
 * Shown while nothing records, the meeting is done processing (a meeting
 * still in the pipeline cannot take more audio yet), and only for a
 * meeting that ended less than two hours ago (UX-08): later than that, a
 * new recording is what people want.
 */
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';

import { type MeetingId, useMeeting } from '@/features/meetings';
import { type StageStatus, usePipelineStatus } from '@/features/pipeline';
import { Button } from '@/ui/button';
import { ICON_PROPS, Mic } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { recordingApi } from '../api';
import { recordingCopy } from '../copy';
import { report, useRecordingState } from '../queries';

/** How long after a meeting ends Resume stays offered. */
export const RESUME_WINDOW_MS = 2 * 60 * 60 * 1000;

/**
 * When the meeting ended, near enough: its pipeline starts the moment the
 * recording stops, so the earliest stage update; without a pipeline run,
 * when the meeting was saved.
 */
export function endedAt(stages: readonly StageStatus[], createdAt: string): number {
  const times = stages.map((stage) => Date.parse(stage.updatedAt)).filter(Number.isFinite);
  return times.length === 0 ? Date.parse(createdAt) : Math.min(...times);
}

/** Whether Resume is offered: processing is over and the meeting ended within the window. */
export function canResume(
  stages: readonly StageStatus[] | undefined,
  createdAt: string | undefined,
  now: number,
): boolean {
  if (stages === undefined || createdAt === undefined) return false;
  const busy = stages.some((stage) => stage.status === 'pending' || stage.status === 'running');
  return !busy && now - endedAt(stages, createdAt) < RESUME_WINDOW_MS;
}

/** The button, or nothing while it cannot be used. */
export function ResumeRecordingButton({ meetingId }: { meetingId: MeetingId }) {
  const navigate = useNavigate();
  const idle = useRecordingState().phase === 'idle';
  const { data: stages } = usePipelineStatus(meetingId);
  const { data: meeting } = useMeeting(meetingId);
  const resume = useMutation({
    mutationFn: () => recordingApi.resumeMeeting(meetingId),
    onSuccess: () => void navigate({ to: '/' }),
    onError: report(recordingCopy.resumeFailed),
  });
  if (!idle || !canResume(stages, meeting?.createdAt, Date.now())) return null;
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
