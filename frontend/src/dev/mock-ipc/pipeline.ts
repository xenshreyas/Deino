/**
 * Mock of `@/lib/ipc/generated/pipeline`: every fixture meeting has a
 * finished `enhance_notes` stage, except the support call, whose notes
 * failed for want of a model so the retry button can be clicked through,
 * and the standup, skipped with automatic notes off (both open on an empty
 * Enhanced view, as `mock-enhanced.ts` seeds them). A re-run walks the
 * stage from pending to running to done, announcing each step with
 * `PipelineProgress`, like the Rust runner. Re-running everything (or the
 * transcript, as ElevenLabs' Retranscribe does) first transcribes again,
 * reporting 0–100% like the `final_transcription` stage, then the notes.
 */
import type * as Generated from '@/lib/ipc/generated/pipeline';

import { mockDatabase } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { mockEvent } from './mock-event';

type MeetingId = Generated.MeetingId;
type StageStatus = Generated.StageStatus;

/** The stage that writes notes, as in `deino_pipeline::registry`. */
const SUMMARY = 'enhance_notes' as Generated.StageId;
/** The stage that transcribes after the meeting, which reports its percent. */
const TRANSCRIPT = 'final_transcription' as Generated.StageId;
/** The re-run stages in run order. */
const ORDER: readonly Generated.StageId[] = [TRANSCRIPT, SUMMARY];
/** The percents a mock transcription reports, one mock round trip apart. */
const PERCENTS: readonly number[] = Array.from({ length: 11 }, (_, step) => step * 10);
/** The fixture whose notes failed. */
const FAILED_MEETING = 'meeting-fixture-customer-support';
/** The fixture whose notes were skipped (`AUTOMATIC_NOTES_OFF` in Rust). */
const SKIPPED_MEETING = 'meeting-fixture-standup';
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
    if (stage !== null && !ORDER.includes(stage)) {
      return failure({ code: 'invalid', message: `the pipeline has no stage named ${stage}` });
    }
    if (stage === SUMMARY) {
      walk(meetingId, SUMMARY, ['pending', 'running', 'succeeded']);
    } else {
      retranscribe(meetingId, stage === null);
    }
    return ok(stagesOf(meetingId));
  },
  async retranscribe(meetingId) {
    await latency();
    const meeting = mockDatabase.meeting(meetingId);
    if (meeting.status === 'error') return failure(meeting.error);
    retranscribe(meetingId, false);
    return ok(stagesOf(meetingId));
  },
} satisfies typeof Generated.commands;

/** The stages after the notes (AI-05, NOT-02's title, ORG-06, ORG-05), as processed. */
const ORGANISE_STAGES: readonly StageStatus[] = [
  {
    stage: 'extract_action_items' as Generated.StageId,
    status: 'succeeded',
    attempts: 1,
    retryAt: null,
    message: null,
    updatedAt: PROCESSED_AT,
  },
  {
    stage: 'auto_title' as Generated.StageId,
    status: 'skipped',
    attempts: 1,
    retryAt: null,
    message: 'The meeting already has a title.',
    updatedAt: PROCESSED_AT,
  },
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
  {
    stage: 'insights' as Generated.StageId,
    status: 'skipped',
    attempts: 1,
    retryAt: null,
    message: 'No insight applies to this meeting.',
    updatedAt: PROCESSED_AT,
  },
];

/** The meeting's stages: re-run ones, else the fixture state. */
function stagesOf(meetingId: MeetingId): StageStatus[] {
  const rerun = stages.get(meetingId);
  const organise = ORGANISE_STAGES.map((stage) => ({ ...stage }));
  if (rerun) return [...rerun.map((stage) => ({ ...stage })), ...organise];
  if (mockDatabase.meeting(meetingId).status === 'error') return [];
  return [fixtureStage(meetingId), ...organise];
}

/** The notes stage as the fixtures were processed. */
function fixtureStage(meetingId: MeetingId | null): StageStatus {
  const done: StageStatus = {
    stage: SUMMARY,
    status: 'succeeded',
    attempts: 1,
    retryAt: null,
    message: null,
    updatedAt: PROCESSED_AT,
  };
  if (meetingId === FAILED_MEETING) {
    const message = 'Add an Anthropic key or pick a local model in Settings → AI.';
    return { ...done, status: 'failed', attempts: 3, message };
  }
  if (meetingId === SKIPPED_MEETING) {
    const message = 'Automatic notes are off in Settings → AI. Use Enhance on the Notes tab.';
    return { ...done, status: 'skipped', message };
  }
  return done;
}

/** A status, or a running stage's percent. */
type Step = Generated.JobStatus | number;

/** Transcribes again with a rising percent, then writes notes when `withNotes`. */
function retranscribe(meetingId: MeetingId, withNotes: boolean): void {
  if (withNotes) report(meetingId, SUMMARY, 'pending');
  const steps: Step[] = ['pending', 'running', ...PERCENTS, 'succeeded'];
  walk(meetingId, TRANSCRIPT, steps, () => {
    if (withNotes) walk(meetingId, SUMMARY, ['running', 'succeeded']);
  });
}

/** Moves `stage` through `steps`, one mock round trip apart, then calls `then`. */
function walk(
  meetingId: MeetingId,
  stage: Generated.StageId,
  steps: readonly Step[],
  then?: () => void,
): void {
  const [step, ...rest] = steps;
  if (step === undefined) {
    then?.();
    return;
  }
  report(meetingId, stage, step);
  void latency().then(() => {
    walk(meetingId, stage, rest, then);
  });
}

/** Records `step` of `stage` and announces it with `PipelineProgress`. */
function report(meetingId: MeetingId, stage: Generated.StageId, step: Step): void {
  if (typeof step === 'number') {
    void events.pipelineProgress.emit({ meetingId, stage, status: 'running', percent: step });
    return;
  }
  const next: StageStatus = {
    ...fixtureStage(null),
    stage,
    status: step,
    attempts: step === 'pending' ? 0 : 1,
    message: null,
    updatedAt: new Date().toISOString(),
  };
  const others = (stages.get(meetingId) ?? []).filter((row) => row.stage !== stage);
  const rows = [...others, next].sort((a, b) => ORDER.indexOf(a.stage) - ORDER.indexOf(b.stage));
  stages.set(meetingId, rows);
  void events.pipelineProgress.emit({ meetingId, stage, status: step, percent: null });
}
