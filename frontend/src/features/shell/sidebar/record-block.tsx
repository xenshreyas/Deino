/**
 * The sidebar's recording area (ui.md §3 Sidebar, "record block"): a Start
 * button while idle; while recording, a live row with level bars, title and
 * timer that opens the record screen, with a Stop button on hover; a quiet
 * "Saving…" row while the last lines are saved. Rust owns the recording
 * (FND-05b, CAP-02); this block follows its state.
 */
import { useNavigate } from '@tanstack/react-router';

import {
  formatElapsed,
  issueLabel,
  type RecordingSnapshot,
  useElapsed,
  useRecordingActions,
  useRecordingState,
} from '@/features/recording';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { IconButton } from '@/ui/icon-button';
import { Square } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { LevelBars } from '@/ui/level-bars';

import { shellCopy } from '../copy';
import { requestRecordingStart } from '../recording';
import { SHORTCUTS } from '../shortcuts';

/** Idle: a full-width secondary button with the shortcut. */
function StartRecordingButton() {
  const navigate = useNavigate();
  return (
    <Button
      variant="secondary"
      className="h-9 w-full justify-start gap-2 px-2.5"
      onClick={() => void requestRecordingStart(() => void navigate({ to: '/' }))}
    >
      <span aria-hidden className="size-[7px] rounded-full bg-recording" />
      <span className="flex-1 text-left">{shellCopy.startRecording}</span>
      <Kbd>{SHORTCUTS.record.display}</Kbd>
    </Button>
  );
}

/** Props of {@link LiveRow}. */
interface LiveRowProps {
  title: string;
  meta: string;
  bars: 'live' | 'paused' | 'idle';
  /** Stops the recording; absent while it is finishing. */
  onStop?: (() => void) | undefined;
}

/** Recording or finishing: an accent-tinted row that opens the live meeting. */
function LiveRow({ title, meta, bars, onStop }: LiveRowProps) {
  const navigate = useNavigate();
  return (
    <div
      className={cn(
        'group flex h-10 w-full items-center rounded-md text-sm transition-colors duration-(--dur-fast)',
        bars === 'idle' ? 'bg-bg-muted' : 'bg-accent-soft hover:brightness-[0.98]',
      )}
    >
      <button
        type="button"
        aria-label={shellCopy.openLiveMeeting}
        onClick={() => void navigate({ to: '/' })}
        className="flex h-full min-w-0 flex-1 items-center gap-2.5 px-2.5"
      >
        <LevelBars state={bars} />
        <span className="min-w-0 flex-1 truncate text-left text-fg">{title}</span>
        <span className="text-xs text-fg-muted tabular-nums">{meta}</span>
      </button>
      {onStop === undefined ? null : (
        <IconButton
          icon={Square}
          label={shellCopy.stopRecording}
          onClick={onStop}
          className="mr-1.5 hidden group-focus-within:inline-flex group-hover:inline-flex"
        />
      )}
    </div>
  );
}

/** A running recording's row, with its ticking timer. */
function RunningRow({ snapshot, title }: { snapshot: RecordingSnapshot; title: string }) {
  const seconds = useElapsed(snapshot);
  const { stop } = useRecordingActions();
  const paused = snapshot.phase === 'paused';
  const issue = snapshot.issues[0];
  return (
    <LiveRow
      title={title}
      meta={
        issue === undefined
          ? paused
            ? shellCopy.paused
            : formatElapsed(seconds)
          : issueLabel(issue)
      }
      bars={paused ? 'paused' : 'live'}
      onStop={() => {
        stop.mutate();
      }}
    />
  );
}

/** The record block: picks the idle, live or finishing presentation. */
export function RecordBlock() {
  const snapshot = useRecordingState();
  const named = snapshot.title?.trim() ?? '';
  const title = named === '' ? shellCopy.untitledRecording : named;
  switch (snapshot.phase) {
    case 'idle':
      return <StartRecordingButton />;
    case 'starting':
      return <LiveRow title={title} meta={shellCopy.starting} bars="idle" />;
    case 'stopping':
      return <LiveRow title={title} meta={shellCopy.saving} bars="idle" />;
    case 'recording':
    case 'paused':
      return <RunningRow key={snapshot.meetingId} snapshot={snapshot} title={title} />;
  }
}
