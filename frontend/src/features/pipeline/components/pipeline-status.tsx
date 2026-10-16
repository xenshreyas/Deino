/**
 * The meeting header's processing line (UX-01): silent when every stage
 * worked or was skipped; one quiet line ("Writing notes…") while work runs,
 * with how far it is when the stage reports that ("Transcribing… · 40%");
 * and one plain-words row per failed stage, with its reason and Retry.
 * A transcript step that gave up on ElevenLabs (CAP-14) gets a row too: it
 * was skipped, not failed (the Mac's transcript was kept and the later steps
 * ran), and its Retry sends the meeting to ElevenLabs again.
 * Every stage, done or not, is in Processing details (the ⋯ menu).
 */
import { toast } from 'sonner';

import { isCloudFailure, useRetranscribe } from '@/features/cloud-transcription';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';

import { copy, stageFailed, stageLabel, stageRunning } from '../copy';
import { usePipelineProgress } from '../hooks/use-pipeline-progress';
import { usePipelineStatus, useRerunStage, useStagePercent } from '../queries';
import type { MeetingId, StageId, StageStatus } from '../types';
import { isStopped, RetryButton } from './stage-item';

interface PipelineStatusProps {
  meetingId: MeetingId;
}

/** The stage the "working" line names: the running one, else the next to run. */
export function activeStage(stages: readonly StageStatus[]): StageStatus | undefined {
  return (
    stages.find((stage) => stage.status === 'running') ??
    stages.find((stage) => stage.status === 'pending')
  );
}

/** The quiet "working" line, with how far the stage is when it says ("Transcribing… · 40%"). */
function RunningLine({ meetingId, stage }: { meetingId: MeetingId; stage: StageId }) {
  const percent = useStagePercent(meetingId, stage);
  return (
    <p role="status" className="animate-pulse-soft text-xs text-fg-muted">
      {stageRunning(stage)}
      {percent === null ? null : <span className="tabular-nums">{copy.percent(percent)}</span>}
    </p>
  );
}

/** One failed stage: what didn't happen, why, and Retry. */
function FailedRow(props: {
  stage: StageStatus;
  retrying: boolean;
  onRetry: (id: StageId) => void;
}) {
  const { stage } = props;
  return (
    <li className="flex min-w-0 items-center gap-1.5 text-xs">
      <CircleAlert {...ICON_PROPS} size={14} className="shrink-0 text-danger" />
      <span className="shrink-0 text-danger">{stageFailed(stage.stage)}</span>
      {stage.message === null ? null : (
        <span className="min-w-0 truncate text-fg-muted" title={stage.message}>
          {stage.message}
        </span>
      )}
      <RetryButton
        label={stageLabel(stage.stage)}
        disabled={props.retrying}
        onRetry={() => {
          props.onRetry(stage.stage);
        }}
      />
    </li>
  );
}

/** The transcript step that gave up on ElevenLabs: its reason, and Retry. */
function CloudFailedRow({ meetingId, stage }: { meetingId: MeetingId; stage: StageStatus }) {
  const retranscribe = useRetranscribe(meetingId);
  return (
    <li className="flex min-w-0 items-center gap-1.5 text-xs">
      <CircleAlert {...ICON_PROPS} size={14} className="shrink-0 text-danger" />
      <span className="min-w-0 truncate text-danger" title={stage.message ?? undefined}>
        {stage.message}
      </span>
      <RetryButton
        label={stageLabel(stage.stage)}
        disabled={retranscribe.isPending}
        onRetry={() => {
          retranscribe.mutate(undefined, {
            onError: (error) => toast.error(copy.retryFailed, { description: error.message }),
          });
        }}
      />
    </li>
  );
}

/**
 * Container: the stages come from a query that `PipelineProgress` events
 * refresh, and retrying is a mutation.
 */
export function PipelineStatus({ meetingId }: PipelineStatusProps) {
  const status = usePipelineStatus(meetingId);
  const rerun = useRerunStage(meetingId);
  usePipelineProgress(meetingId);
  const stages = status.data ?? [];
  const failed = stages.filter(isStopped);
  const cloud = stages.find(isCloudFailure);
  const active = activeStage(stages);
  const quiet = failed.length === 0 && cloud === undefined;
  if (quiet && active === undefined && !rerun.isError) return null;
  return (
    <section aria-label={copy.label} className="flex flex-col gap-1">
      {quiet ? null : (
        <ul className="flex flex-col gap-0.5">
          {cloud === undefined ? null : <CloudFailedRow meetingId={meetingId} stage={cloud} />}
          {failed.map((stage) => (
            <FailedRow
              key={stage.stage}
              stage={stage}
              retrying={rerun.isPending}
              onRetry={(id) => {
                rerun.mutate(id);
              }}
            />
          ))}
        </ul>
      )}
      {failed.length === 0 && active !== undefined ? (
        <RunningLine meetingId={meetingId} stage={active.stage} />
      ) : null}
      {rerun.isError ? (
        <p role="alert" className="text-xs text-danger">
          {rerun.error.message || copy.retryFailed}
        </p>
      ) : null}
    </section>
  );
}
