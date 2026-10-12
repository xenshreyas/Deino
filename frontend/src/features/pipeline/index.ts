/**
 * The post-meeting pipeline: each meeting's processing stages, their status
 * and re-runs, through the `pipeline` plugin. Other code imports from here only.
 *
 * `PipelineStatus` is the meeting header's quiet line (only while work runs
 * or after a failure); `ProcessingDetailsDialog` lists every stage, from
 * the meeting's ⋯ menu.
 */
export { pipelineApi } from './api';
export { PipelineStatus } from './components/pipeline-status';
export {
  ProcessingDetailsDialog,
  type ProcessingDetailsDialogProps,
} from './components/processing-details';
export { KNOWN_STAGES, type KnownStage, stageLabel } from './copy';
export { usePipelineProgress } from './hooks/use-pipeline-progress';
export { pipelineKeys, pipelineStatusQuery, usePipelineStatus, useRerunStage } from './queries';
export { NOTES_STAGE } from './types';
export type { JobStatus, PipelineProgress, StageId, StageStatus } from './types';
