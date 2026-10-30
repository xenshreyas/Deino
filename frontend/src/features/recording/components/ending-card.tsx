/**
 * Auto-stop's "Ending…" notice (CAP-07) as the top-centre card the meeting
 * prompt uses: "Recording ends in 12s" over why, Keep recording and Stop
 * now, and the time left draining red along the bottom. It shares the
 * prompt's window, which Rust shows while the notice is up. No close
 * button: closing would look like keeping the recording, yet it stops.
 */
import { useState } from 'react';

import { TOP_BUTTON, TopCard, TopCardHeading, useTopExit } from '@/ui/top-card';

import { endingReason, recordingCopy } from '../copy';
import { useAutoStop, useAutoStopAnswers } from '../hooks/use-auto-stop';
import { useSecondsUntil } from '../hooks/use-record-panel';
import { useRecordingActions } from '../queries';
import type { AutoStopState } from '../types';

/** The notice being shown. */
type Ending = NonNullable<AutoStopState['ending']>;

/** Props of {@link EndingCardView}. */
export interface EndingCardViewProps {
  ending: Ending;
  /** Seconds until the recording stops. */
  seconds: number;
  busy: boolean;
  onKeep: () => void;
  onStopNow: () => void;
}

/** The card (presentational). */
export function EndingCardView(props: EndingCardViewProps) {
  const [leaving, leave] = useTopExit(false);
  const [drainMs] = useState(() => Math.max(0, Date.parse(props.ending.stopAt) - Date.now()));
  return (
    <TopCard
      label={recordingCopy.endingLabel}
      role="status"
      leaving={leaving}
      drainMs={drainMs}
      tone="danger"
    >
      <TopCardHeading
        title={recordingCopy.endsIn(props.seconds)}
        detail={<span className="truncate">{endingReason[props.ending.reason]}</span>}
      />
      <div className="mt-3 flex gap-2">
        <button
          type="button"
          disabled={props.busy}
          onClick={() => {
            leave(props.onKeep);
          }}
          className={TOP_BUTTON.primary}
        >
          {recordingCopy.keepRecording}
        </button>
        <button
          type="button"
          disabled={props.busy}
          onClick={() => {
            leave(props.onStopNow);
          }}
          className={TOP_BUTTON.quiet}
        >
          {recordingCopy.stopNow}
        </button>
      </div>
    </TopCard>
  );
}

/** The card while auto-stop's notice is up; nothing otherwise. */
export function EndingCard() {
  const autoStop = useAutoStop();
  const answers = useAutoStopAnswers();
  const { stop } = useRecordingActions();
  const seconds = useSecondsUntil(autoStop.ending?.stopAt ?? null);
  if (autoStop.ending === null || seconds === null) return null;
  return (
    <EndingCardView
      key={autoStop.ending.stopAt}
      ending={autoStop.ending}
      seconds={seconds}
      busy={answers.keep.isPending || stop.isPending}
      onKeep={() => {
        answers.keep.mutate();
      }}
      onStopNow={() => {
        stop.mutate();
      }}
    />
  );
}
