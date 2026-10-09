import { usePipelineProgress } from '../hooks/use-pipeline-progress';
import { copy } from '../copy';
import { usePipelineStatus, useRerunStage } from '../queries';
import type { MeetingId } from '../types';
import { StageItem } from './stage-item';

interface PipelineStripProps {
  meetingId: MeetingId;
}

/**
 * The meeting's processing status: each post-meeting stage with its status,
 * and a retry button on a failed one. Renders nothing for a meeting the
 * pipeline never ran for. Container: the stages come from a query that
 * `PipelineProgress` events refresh, and retrying is a mutation.
 */
export function PipelineStrip({ meetingId }: PipelineStripProps) {
  const status = usePipelineStatus(meetingId);
  const rerun = useRerunStage(meetingId);
  usePipelineProgress(meetingId);

  if (status.isError) {
    return <p className="text-xs text-fg-subtle">{copy.loadFailed}</p>;
  }
  const stages = status.data ?? [];
  if (stages.length === 0) return null;
  return (
    <section aria-label={copy.label} className="flex flex-col gap-1">
      <ul className="flex flex-wrap items-center gap-x-4 gap-y-1">
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
    </section>
  );
}
