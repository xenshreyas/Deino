/**
 * What the pill reveals on hover (ui.md, Record pill, Expanded): pause or
 * resume, stop, "Open notes", and ⋯ for the rare device switches (UX-29:
 * microphone, screen to capture, the call's audio); and, while auto-stop
 * counts down, "Keep recording" instead.
 * Presentational: props in, events out.
 */
import { Button } from '@/ui/button';
import { IconButton } from '@/ui/icon-button';
import { Ellipsis, NotebookPen, Pause, Play } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { panelCopy } from '../../copy';

/** The pill's actions. */
export interface PillActions {
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  /** The ⋯ menu: microphone, screen to capture, the call's audio. */
  onMore: () => void;
  onOpenNotes: () => void;
  onKeep: () => void;
}

/** A hairline between the status and the controls. */
export function Divider() {
  return <span aria-hidden className="mx-1 h-4 w-px shrink-0 bg-border" />;
}

/** Stop: a 28px circle around the recording square. */
function StopButton({ onStop }: { onStop: () => void }) {
  return (
    <Hint label={panelCopy.stop} side="bottom">
      <button
        type="button"
        aria-label={panelCopy.stop}
        onClick={onStop}
        className="flex size-7 shrink-0 items-center justify-center rounded-full transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:ring-2 focus-visible:ring-accent focus-visible:outline-none"
      >
        <span className="size-2.5 rounded-[2px] bg-recording" />
      </button>
    </Hint>
  );
}

/** Props of {@link PillControls}. */
export interface PillControlsProps {
  paused: boolean;
  actions: PillActions;
}

/** The running recording's controls. */
export function PillControls({ paused, actions }: PillControlsProps) {
  return (
    <span className="flex items-center gap-0.5">
      <Divider />
      <IconButton
        icon={paused ? Play : Pause}
        label={paused ? panelCopy.resume : panelCopy.pause}
        onClick={paused ? actions.onResume : actions.onPause}
      />
      <StopButton onStop={actions.onStop} />
      <Hint label={panelCopy.openNotesHint} side="bottom">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1.5 px-2 text-xs"
          onClick={actions.onOpenNotes}
        >
          <NotebookPen size={14} strokeWidth={1.5} aria-hidden />
          {panelCopy.openNotes}
        </Button>
      </Hint>
      <IconButton icon={Ellipsis} label={panelCopy.more} onClick={actions.onMore} />
    </span>
  );
}

/** Auto-stop's countdown and "Keep recording". */
export function EndingControls({
  seconds,
  reason,
  onKeep,
}: {
  seconds: number;
  reason: string;
  onKeep: () => void;
}) {
  return (
    <span className="flex items-center gap-2" role="status" aria-live="polite">
      <Divider />
      <span className="text-xs whitespace-nowrap text-fg-muted tabular-nums" title={reason}>
        {panelCopy.ending(seconds)}
      </span>
      <Button variant="primary" size="sm" className="h-7 px-2.5 text-xs" onClick={onKeep}>
        {panelCopy.keepRecording}
      </Button>
    </span>
  );
}
