/**
 * Mock of `@/lib/ipc/generated/pipeline`: every fixture meeting has a
 * finished `legacy_summary` stage, except the one-on-one, whose summary
 * failed so the retry button can be clicked through. A re-run walks the
 * stage from pending to running to done, announcing each step with
 * `PipelineProgress`, like the Rust runner.
 */
import type * as Generated from '@/lib/ipc/generated/pipeline';

import { mockDatabase } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { mockEvent } from './mock-event';

type MeetingId = Generated.MeetingId;
type StageStatus = Generated.StageStatus;

/** The one registered stage, as in `deino_pipeline::registry`. */
const SUMMARY = 'legacy_summary' as Generated.StageId;
/** The fixture whose summary failed. */
const FAILED_MEETING = 'meeting-fixture-one-on-one';
/** When the fixtures were processed. */
const PROCESSED_AT = '2026-10-09T09:00:00.000Z';

/** Stages the mock has re-run, by meeting; others show their fixture state. */
const stages = new Map<MeetingId, StageStatus[]>();

export const events = {
  pipelineProgress: mockEvent<Generated.PipelineProgress>('plugin:pipeline:pipeline-progress'),
} satisfies typeof Generated.events;

export const commands = {
  async status(meetingId) {
    await latency();
    return ok(stagesOf(meetingId));
  },
  async rerun(meetingId, stage) {
    await latency();
    const meeting = mockDatabase.meeting(meetingId);
    if (meeting.status === 'error') return failure(meeting.error);
    if (stage !== null && stage !== SUMMARY) {
      return failure({ code: 'invalid', message: `the pipeline has no stage named ${stage}` });
    }
    advance(meetingId, ['pending', 'running', 'succeeded']);
    return ok(stagesOf(meetingId));
  },
} satisfies typeof Generated.commands;

/** The organising stages after the summary (ORG-06, ORG-05), as processed. */
const ORGANISE_STAGES: readonly StageStatus[] = [
  {
    stage: 'auto_tag' as Generated.StageId,
    status: 'succeeded',
    attempts: 1,
    retryAt: null,
    message: null,
    updatedAt: PROCESSED_AT,
  },
  {
    stage: 'auto_folder' as Generated.StageId,
    status: 'skipped',
    attempts: 1,
    retryAt: null,
    message: 'No folder has auto-add rules.',
    updatedAt: PROCESSED_AT,
  },
];

/** The meeting's stages: re-run ones, else the fixture state. */
function stagesOf(meetingId: MeetingId): StageStatus[] {
  const rerun = stages.get(meetingId);
  const organise = ORGANISE_STAGES.map((stage) => ({ ...stage }));
  if (rerun) return [...rerun.map((stage) => ({ ...stage })), ...organise];
  if (mockDatabase.meeting(meetingId).status === 'error') return [];
  return [fixtureStage(meetingId === FAILED_MEETING), ...organise];
}

function fixtureStage(failed: boolean): StageStatus {
  return {
    stage: SUMMARY,
    status: failed ? 'failed' : 'succeeded',
    attempts: failed ? 3 : 1,
    retryAt: null,
    message: failed ? 'The summary could not be generated; the summary panel shows why.' : null,
    updatedAt: PROCESSED_AT,
  };
}

/** Moves the summary stage through `steps`, one mock round trip apart. */
function advance(meetingId: MeetingId, steps: readonly Generated.JobStatus[]): void {
  const [status, ...rest] = steps;
  if (status === undefined) return;
  const attempts = status === 'pending' ? 0 : 1;
  const stage: StageStatus = {
    ...fixtureStage(false),
    status,
    attempts,
    message: null,
    updatedAt: new Date().toISOString(),
  };
  stages.set(meetingId, [stage]);
  void events.pipelineProgress.emit({ meetingId, stage: SUMMARY, status });
  if (rest.length > 0) {
    void latency().then(() => {
      advance(meetingId, rest);
    });
  }
}
