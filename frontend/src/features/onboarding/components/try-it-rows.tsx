import type { MeetingId } from '@/features/meetings';
import { type RecordingSnapshot, useElapsed } from '@/features/recording';
import { useMountEffect } from '@/hooks/useMountEffect';
import { IpcError } from '@/lib/ipc/result';
import { Button } from '@/ui/button';
import { CircleCheck, FileText, ICON_PROPS, Mic } from '@/ui/icons';
import { LevelBars } from '@/ui/level-bars';

import { tryItCopy } from '../copy';
import { useImportSample } from '../queries';
import { secondsLeft, type TestView } from '../test-recording';
import { SetupRow } from './setup-row';

/** Props of {@link TestRow}. */
export interface TestRowProps {
  view: TestView;
  snapshot: RecordingSnapshot;
  onStart: () => void;
  onStop: () => void;
}

/** The 20-second test: start it, watch it count down, see it saved. */
export function TestRow({ view, snapshot, onStart, onStop }: TestRowProps) {
  if (view.kind === 'recording') return <Countdown snapshot={snapshot} onStop={onStop} />;
  const failed = view.kind === 'failed';
  return (
    <SetupRow icon={Mic} name={tryItCopy.testName} detail={<TestDetail view={view} />}>
      {view.kind === 'done' ? (
        <span className="flex items-center gap-1.5 text-sm text-fg-muted">
          <CircleCheck {...ICON_PROPS} className="text-accent" />
          {tryItCopy.ready}
        </span>
      ) : (
        <Button variant="accent" size="sm" disabled={view.kind === 'saving'} onClick={onStart}>
          {failed ? tryItCopy.retry : tryItCopy.start}
        </Button>
      )}
    </SetupRow>
  );
}

/** The test row's second line for every state but recording. */
function TestDetail({ view }: { view: TestView }) {
  switch (view.kind) {
    case 'failed':
      return (
        <span role="alert" className="text-danger">
          {tryItCopy.failed} {view.message}
        </span>
      );
    case 'saving':
      return tryItCopy.saving;
    case 'done':
      return tryItCopy.done;
    case 'idle':
    case 'recording':
      return tryItCopy.testWhy;
  }
}

/** Live level bars and the seconds left; stops the recorder at zero. */
function Countdown({ snapshot, onStop }: { snapshot: RecordingSnapshot; onStop: () => void }) {
  const left = secondsLeft(useElapsed(snapshot));
  const live = snapshot.phase === 'recording';
  const detail = (
    <span className="text-fg tabular-nums" aria-live="polite">
      {live ? tryItCopy.remaining(left) : tryItCopy.recording}
    </span>
  );
  return (
    <SetupRow icon={Mic} name={tryItCopy.testName} detail={detail}>
      <LevelBars state={live ? 'live' : 'idle'} />
      <Button variant="secondary" size="sm" onClick={onStop} disabled={!live}>
        {tryItCopy.stop}
      </Button>
      {live && left === 0 ? <StopOnMount onStop={onStop} /> : null}
    </SetupRow>
  );
}

/** Stops the test once, when the countdown reaches zero. */
function StopOnMount({ onStop }: { onStop: () => void }) {
  useMountEffect(onStop);
  return null;
}

/** Props of {@link SampleRow}. */
export interface SampleRowProps {
  /** Off while the test is recording. */
  disabled: boolean;
  /** Opens the imported sample (and ends onboarding). */
  onOpen: (meetingId: MeetingId) => void;
}

/** The bundled sample meeting: imported once, then opened. */
export function SampleRow({ disabled, onOpen }: SampleRowProps) {
  const sample = useImportSample();
  const detail =
    sample.error === null ? (
      tryItCopy.sampleWhy
    ) : (
      <span role="alert" className="text-danger">
        {tryItCopy.sampleFailed} {IpcError.from(sample.error).message}
      </span>
    );
  return (
    <SetupRow icon={FileText} name={tryItCopy.sampleName} detail={detail}>
      <Button
        variant="secondary"
        size="sm"
        disabled={disabled || sample.isPending}
        onClick={() => {
          sample.mutate(undefined, { onSuccess: onOpen });
        }}
      >
        {sample.isPending ? tryItCopy.sampleOpening : tryItCopy.sampleOpen}
      </Button>
    </SetupRow>
  );
}
