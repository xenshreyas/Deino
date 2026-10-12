import { useNavigate } from '@tanstack/react-router';

import type { MeetingId } from '@/features/meetings';
import { type RecordingSnapshot, useElapsed } from '@/features/recording';
import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { CircleCheck, ICON_PROPS, Mic } from '@/ui/icons';
import { LevelBars } from '@/ui/level-bars';

import { tryItCopy } from '../../copy';
import { useTestRecording } from '../../hooks/use-test-recording';
import { secondsLeft, type TestView } from '../../test-recording';
import { type PrimaryAction, StepFrame, type StepProps } from '../step-frame';

/**
 * Step 6: a 20-second test through the real recorder. When it stops, the
 * test is a meeting like any other, and the main action opens it.
 */
export function TryItStep({ dispatch }: StepProps) {
  const test = useTestRecording();
  const navigate = useNavigate();
  const open = (meetingId: MeetingId) => {
    dispatch({ type: 'continue' });
    void navigate({ to: '/meetings/$meetingId', params: { meetingId } });
  };
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
      <TestPanel
        view={test.view}
        snapshot={test.snapshot}
        onStart={test.start}
        onStop={test.stop}
      />
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

interface TestPanelProps {
  view: TestView;
  snapshot: RecordingSnapshot;
  onStart: () => void;
  onStop: () => void;
}

/** The test itself: a start button, the live countdown, or the result. */
function TestPanel({ view, snapshot, onStart, onStop }: TestPanelProps) {
  switch (view.kind) {
    case 'recording':
      return <Countdown snapshot={snapshot} onStop={onStop} />;
    case 'saving':
      return <p className="text-sm text-fg-muted">{tryItCopy.saving}</p>;
    case 'done':
      return (
        <p className="flex items-center gap-2 text-sm text-fg">
          <CircleCheck {...ICON_PROPS} className="text-accent" />
          {tryItCopy.done}
        </p>
      );
    case 'idle':
    case 'failed':
      return <StartTest failed={view.kind === 'failed' ? view.message : null} onStart={onStart} />;
  }
}

/** The start button, with the last failure under it. */
function StartTest({ failed, onStart }: { failed: string | null; onStart: () => void }) {
  return (
    <div className="flex flex-col items-start gap-3">
      <Button variant="accent" size="lg" onClick={onStart}>
        <Mic {...ICON_PROPS} />
        {tryItCopy.start}
      </Button>
      {failed === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {tryItCopy.failed} {failed}
        </p>
      )}
    </div>
  );
}

/** Live level bars and the seconds left; stops the recorder at zero. */
function Countdown({ snapshot, onStop }: { snapshot: RecordingSnapshot; onStop: () => void }) {
  const left = secondsLeft(useElapsed(snapshot));
  const live = snapshot.phase === 'recording';
  return (
    <div className="flex items-center gap-4">
      <LevelBars state={live ? 'live' : 'idle'} />
      <span className="text-sm text-fg tabular-nums" aria-live="polite">
        {live ? tryItCopy.remaining(left) : tryItCopy.recording}
      </span>
      <span className="flex-1" />
      <Button variant="secondary" size="sm" onClick={onStop} disabled={!live}>
        {tryItCopy.stop}
      </Button>
      {live && left === 0 ? <StopOnMount onStop={onStop} /> : null}
    </div>
  );
}

/** Stops the test once, when the countdown reaches zero. */
function StopOnMount({ onStop }: { onStop: () => void }) {
  useMountEffect(onStop);
  return null;
}
