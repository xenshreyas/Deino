/**
 * "Try Deino" (UX-24): the test recording and the sample meeting that used
 * to end setup, now on Home while there are no notes yet. Either opens a
 * real meeting page.
 */
import { useNavigate } from '@tanstack/react-router';

import type { MeetingId } from '@/features/meetings';

import { tryItCopy } from '../copy';
import { useTestRecording } from '../hooks/use-test-recording';
import { SetupList } from './setup-row';
import { SampleRow, TestRow } from './try-it-rows';

/** The two rows; the sample waits while the test records. */
export function TryDeino() {
  const test = useTestRecording();
  const navigate = useNavigate();
  const open = (meetingId: MeetingId) => {
    void navigate({ to: '/meetings/$meetingId', params: { meetingId } });
  };
  const busy = test.view.kind === 'recording' || test.view.kind === 'saving';
  return (
    <SetupList label={tryItCopy.title}>
      <TestRow
        view={test.view}
        snapshot={test.snapshot}
        onStart={test.start}
        onStop={test.stop}
        onOpen={open}
      />
      <SampleRow disabled={busy} onOpen={open} />
    </SetupList>
  );
}
