import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

import { recordingApi, type RecordingSnapshot, useRecordingState } from '@/features/recording';
import { IpcError } from '@/lib/ipc/result';

import { tryItCopy } from '../copy';
import { type TestOutcome, type TestView, testView } from '../test-recording';

/** The test recording: what to show, and how to start and stop it. */
export interface TestRecording {
  view: TestView;
  /** The recorder, for the countdown. */
  snapshot: RecordingSnapshot;
  start: () => void;
  stop: () => void;
}

/** A failure's message for the step. */
function failure(error: unknown): TestOutcome {
  return { kind: 'failed', message: IpcError.from(error).message };
}

/**
 * Records a short test through the real recorder (`recording` plugin), so
 * the first meeting is a real one: transcribed on this Mac and turned into
 * notes by the pipeline like any other.
 */
export function useTestRecording(): TestRecording {
  const snapshot = useRecordingState();
  const [outcome, setOutcome] = useState<TestOutcome>({ kind: 'none' });
  const start = useMutation({
    mutationFn: () => recordingApi.start(tryItCopy.testTitle),
    onMutate: () => {
      setOutcome({ kind: 'none' });
    },
    onError: (error) => {
      setOutcome(failure(error));
    },
  });
  const stop = useMutation({
    mutationFn: recordingApi.stop,
    onSuccess: (meetingId) => {
      setOutcome({ kind: 'done', meetingId });
    },
    onError: (error) => {
      setOutcome(failure(error));
    },
  });
  return {
    view: start.isPending ? { kind: 'recording' } : testView(snapshot, outcome),
    snapshot,
    start: () => {
      start.mutate();
    },
    stop: () => {
      if (!stop.isPending) stop.mutate();
    },
  };
}
