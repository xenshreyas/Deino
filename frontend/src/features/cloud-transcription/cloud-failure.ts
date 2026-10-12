/**
 * Which processing step gave up on ElevenLabs (CAP-14). The transcript step
 * is skipped, not failed, when ElevenLabs fails for good: the Mac's
 * transcript stays and the later steps run. The meeting header's status
 * line shows it with Retry (`features/pipeline`'s `PipelineStatus`).
 */
import { CLOUD_FAILED } from './copy';
import type { StageStatus } from './types';

/** Whether `stage` is the transcript step that gave up on ElevenLabs. */
export function isCloudFailure(stage: StageStatus): boolean {
  return (
    stage.stage === 'final_transcription' &&
    stage.status === 'skipped' &&
    (stage.message?.startsWith(CLOUD_FAILED) ?? false)
  );
}
