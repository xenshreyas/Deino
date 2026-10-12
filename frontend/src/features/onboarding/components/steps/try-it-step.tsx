import { useNavigate } from '@tanstack/react-router';

import type { MeetingId } from '@/features/meetings';

import { tryItCopy } from '../../copy';
import { useTestRecording } from '../../hooks/use-test-recording';
import type { TestView } from '../../test-recording';
import { SetupList } from '../setup-row';
import { type PrimaryAction, StepFrame, type StepProps } from '../step-frame';
import { SampleRow, TestRow } from '../try-it-rows';

/**
 * Step 6: a 20-second test through the real recorder, or the bundled sample
 * meeting. Either way the user ends on a meeting page with real notes, and
 * onboarding is finished.
 */
export function TryItStep({ dispatch }: StepProps) {
  const test = useTestRecording();
  const navigate = useNavigate();
  const open = (meetingId: MeetingId) => {
    dispatch({ type: 'continue' });
    void navigate({ to: '/meetings/$meetingId', params: { meetingId } });
  };
  const busy = test.view.kind === 'recording' || test.view.kind === 'saving';
  return (
    <StepFrame
      step="try_it"
      title={tryItCopy.title}
      description={tryItCopy.description}
      dispatch={dispatch}
      primary={primaryFor(test.view, open, () => {
        dispatch({ type: 'continue' });
      })}
    >
      <SetupList label={tryItCopy.title}>
        <TestRow
          view={test.view}
          snapshot={test.snapshot}
          onStart={test.start}
          onStop={test.stop}
        />
        <SampleRow disabled={busy} onOpen={open} />
      </SetupList>
    </StepFrame>
  );
}

/** "Open it" once the test is saved; otherwise "Finish", disabled mid-test. */
function primaryFor(
  view: TestView,
  open: (meetingId: MeetingId) => void,
  finish: () => void,
): PrimaryAction {
  if (view.kind === 'done')
    return {
      label: tryItCopy.open,
      onClick: () => {
        open(view.meetingId);
      },
    };
  const busy = view.kind === 'recording' || view.kind === 'saving';
  return { label: tryItCopy.finish, onClick: finish, disabled: busy };
}
