/**
 * Retry beside the transcript step's "ElevenLabs transcription failed: …"
 * status (CAP-14). The step was skipped, not failed, so the transcript
 * made on this Mac stayed and the later steps ran; Retry sends the meeting
 * to ElevenLabs again and re-runs everything after it.
 */
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { ICON_PROPS, RotateCcw } from '@/ui/icons';

import { CLOUD_FAILED, cloudCopy } from '../copy';
import { useRetranscribe } from '../queries';
import type { MeetingId, StageStatus } from '../types';

/** Whether `stage` is the transcript step that gave up on ElevenLabs. */
export function isCloudFailure(stage: StageStatus): boolean {
  return (
    stage.stage === 'final_transcription' &&
    stage.status === 'skipped' &&
    (stage.message?.startsWith(CLOUD_FAILED) ?? false)
  );
}

/** Props of {@link CloudRetry}. */
export interface CloudRetryProps {
  meetingId: MeetingId;
  stage: StageStatus;
}

/** Retry for a step that gave up on ElevenLabs; nothing for any other. */
export function CloudRetry({ meetingId, stage }: CloudRetryProps) {
  const retranscribe = useRetranscribe(meetingId);
  if (!isCloudFailure(stage)) return null;
  return (
    <Button
      variant="ghost"
      size="sm"
      className="h-6 px-1.5 text-xs"
      aria-label={cloudCopy.retryLabel}
      disabled={retranscribe.isPending}
      onClick={() => {
        retranscribe.mutate(undefined, {
          onError: (error) =>
            toast.error(cloudCopy.retranscribeFailed, { description: error.message }),
        });
      }}
    >
      <RotateCcw {...ICON_PROPS} size={14} />
      {cloudCopy.retry}
    </Button>
  );
}
