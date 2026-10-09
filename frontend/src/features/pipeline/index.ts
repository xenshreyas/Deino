/**
 * The post-meeting pipeline: each meeting's processing stages, their status
 * and re-runs, through the `pipeline` plugin. Other code imports from here only.
 *
 * `PipelineStrip` is mounted in the meeting page header for now; FND-10's
 * meeting page gives it its final place.
 */
export { pipelineApi } from './api';
export { PipelineStrip } from './components/pipeline-strip';
export { usePipelineProgress } from './hooks/use-pipeline-progress';
export { pipelineKeys, pipelineStatusQuery, usePipelineStatus, useRerunStage } from './queries';
export type { JobStatus, PipelineProgress, StageId, StageStatus } from './types';
