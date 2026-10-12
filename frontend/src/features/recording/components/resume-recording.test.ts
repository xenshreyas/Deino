import { describe, expect, it } from 'vitest';

import type { JobStatus, StageId, StageStatus } from '@/features/pipeline';

import { canResume, RESUME_WINDOW_MS } from './resume-recording';

const ENDED = '2026-10-12T10:00:00Z';
const CREATED = '2026-10-12T09:00:00Z';

function stage(status: JobStatus, updatedAt = ENDED): StageStatus {
  return {
    stage: 'finalize_audio' as StageId,
    status,
    attempts: 1,
    retryAt: null,
    message: null,
    updatedAt,
  };
}

const at = (iso: string, plusMs = 0) => Date.parse(iso) + plusMs;

describe('Resume recording (UX-08)', () => {
  it('is offered for a processed meeting that ended under two hours ago', () => {
    const later = stage('succeeded', '2026-10-12T10:05:00Z');
    expect(canResume([stage('succeeded'), later], CREATED, at(ENDED, 60_000))).toBe(true);
  });

  it('is not offered two hours after the meeting ended', () => {
    expect(canResume([stage('succeeded')], CREATED, at(ENDED, RESUME_WINDOW_MS))).toBe(false);
  });

  it('is not offered while the meeting is still processing', () => {
    expect(canResume([stage('succeeded'), stage('running')], CREATED, at(ENDED))).toBe(false);
  });

  it('falls back to when the meeting was saved when nothing was processed', () => {
    expect(canResume([], CREATED, at(CREATED, 60_000))).toBe(true);
    expect(canResume([], CREATED, at(CREATED, RESUME_WINDOW_MS + 1))).toBe(false);
  });

  it('waits for both the stages and the meeting to load', () => {
    expect(canResume(undefined, CREATED, at(ENDED))).toBe(false);
    expect(canResume([], undefined, at(ENDED))).toBe(false);
  });
});
