/**
 * The sidebar's recording area (ui.md §3 Sidebar, "record block"): a Start
 * button while idle, a live row with level bars, title and timer while
 * recording, and a quiet "Saving…" row while the recording is finalised.
 *
 * Stopping stays on the record screen, which owns the stop-and-save flow
 * until FND-05/CAP-10 move it to Rust; the live row opens that screen.
 */
import { useNavigate, useRouterState } from '@tanstack/react-router';

import { RecordingStatus, useRecordingState } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { Kbd } from '@/ui/kbd';
import { LevelBars } from '@/ui/level-bars';

import { shellCopy } from '../copy';
import { formatElapsed } from '../format';
import { requestRecordingStart } from '../recording';
import { SHORTCUTS } from '../shortcuts';

/** Statuses after Stop while the recording is being finalised and saved. */
const FINISHING: ReadonlySet<RecordingStatus> = new Set([
  RecordingStatus.STOPPING,
  RecordingStatus.PROCESSING_TRANSCRIPTS,
  RecordingStatus.SAVING,
]);

/** Idle: a full-width secondary button with the shortcut. */
function StartRecordingButton() {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  return (
    <Button
      variant="secondary"
      className="h-9 w-full justify-start gap-2 px-2.5"
      onClick={() => {
        requestRecordingStart(pathname, () => void navigate({ to: '/' }));
      }}
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
}

/** Recording or finishing: an accent-tinted row that opens the live meeting. */
function LiveRow({ title, meta, bars }: LiveRowProps) {
  const navigate = useNavigate();
  return (
    <button
      type="button"
      aria-label={shellCopy.openLiveMeeting}
      onClick={() => void navigate({ to: '/' })}
      className={cn(
        'flex h-10 w-full items-center gap-2.5 rounded-md px-2.5 text-sm transition-colors duration-(--dur-fast)',
        bars === 'idle' ? 'bg-bg-muted' : 'bg-accent-soft hover:brightness-[0.98]',
      )}
    >
      <LevelBars state={bars} />
      <span className="min-w-0 flex-1 truncate text-left text-fg">{title}</span>
      <span className="text-xs text-fg-muted tabular-nums">{meta}</span>
    </button>
  );
}

/** The record block: picks the idle, live or finishing presentation. */
export function RecordBlock() {
  const { isRecording, isPaused, activeDuration, status } = useRecordingState();
  const { meetingTitle } = useTranscripts();
  const title = meetingTitle.trim() === '' ? shellCopy.untitledRecording : meetingTitle;

  if (FINISHING.has(status)) return <LiveRow title={title} meta={shellCopy.saving} bars="idle" />;
  if (!isRecording) return <StartRecordingButton />;
  return (
    <LiveRow
      title={title}
      meta={isPaused ? shellCopy.paused : formatElapsed(activeDuration)}
      bars={isPaused ? 'paused' : 'live'}
    />
  );
}
