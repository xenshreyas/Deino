/**
 * The record screen's control bar: Start while idle; level bars, the timer,
 * Pause or Resume and Stop while recording; a quiet "Saving…" while the
 * last lines are saved. Every action goes to Rust; the bar follows the
 * recorder's state events.
 */
import { Button } from '@/ui/button';
import { ICON_PROPS, Pause, Play, Square } from '@/ui/icons';
import { LevelBars } from '@/ui/level-bars';

import { issueLabel, recordingCopy } from '../copy';
import { formatElapsed } from '../elapsed';
import { useElapsed } from '../hooks/use-elapsed';
import { useRecordingActions, useRecordingState } from '../queries';
import type { RecordingSnapshot } from '../types';

const BAR =
  'flex h-11 items-center gap-3 rounded-full border border-border bg-surface-raised px-4 shadow-md';

type Actions = ReturnType<typeof useRecordingActions>;

/** The first open health issue, in the warning colour (CAP-10). */
function HealthLabel({ snapshot }: { snapshot: RecordingSnapshot }) {
  const issue = snapshot.issues[0];
  if (issue === undefined) return null;
  return (
    <span role="alert" className="text-xs whitespace-nowrap text-warning-text">
      {issueLabel(issue)}
    </span>
  );
}

/** Recording or paused: the timer and the pause and stop buttons. */
function Running({ snapshot, actions }: { snapshot: RecordingSnapshot; actions: Actions }) {
  const seconds = useElapsed(snapshot);
  const paused = snapshot.phase === 'paused';
  const toggle = paused ? actions.resume : actions.pause;
  return (
    <div className={BAR}>
      <LevelBars state={paused ? 'paused' : 'live'} />
      <span className="min-w-12 text-sm text-fg tabular-nums">
        {paused ? recordingCopy.paused : formatElapsed(seconds)}
      </span>
      <HealthLabel snapshot={snapshot} />
      <Button
        variant="ghost"
        size="sm"
        disabled={toggle.isPending}
        onClick={() => {
          toggle.mutate();
        }}
      >
        {paused ? <Play {...ICON_PROPS} /> : <Pause {...ICON_PROPS} />}
        {paused ? recordingCopy.resume : recordingCopy.pause}
      </Button>
      <Button
        variant="danger-solid"
        size="sm"
        disabled={actions.stop.isPending}
        onClick={() => {
          actions.stop.mutate();
        }}
      >
        <Square {...ICON_PROPS} />
        {recordingCopy.stop}
      </Button>
    </div>
  );
}

/** The bar for the recorder's current phase. */
export function RecordingControls() {
  const snapshot = useRecordingState();
  const actions = useRecordingActions();
  switch (snapshot.phase) {
    case 'idle':
      return (
        <Button
          variant="accent"
          size="lg"
          disabled={actions.start.isPending}
          onClick={() => {
            actions.start.mutate();
          }}
        >
          <span aria-hidden className="size-[7px] rounded-full bg-recording" />
          {recordingCopy.start}
        </Button>
      );
    case 'starting':
    case 'stopping':
      return (
        <div className={BAR}>
          <LevelBars state="idle" />
          <span className="text-sm text-fg-muted">
            {snapshot.phase === 'starting' ? recordingCopy.starting : recordingCopy.stopping}
          </span>
        </div>
      );
    case 'recording':
    case 'paused':
      return <Running key={snapshot.meetingId} snapshot={snapshot} actions={actions} />;
  }
}
