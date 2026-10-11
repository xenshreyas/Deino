/**
 * What the pill reveals on hover (ui.md, Record pill, Expanded): pause or
 * resume, stop, the microphone menu, the call-audio switch and "Open
 * notes"; and, while auto-stop counts down, "Keep recording" instead.
 * Presentational: props in, events out.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { IconButton } from '@/ui/icon-button';
import { Mic, NotebookPen, Pause, Play, Volume2, VolumeX } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { panelCopy } from '../../copy';

/** The pill's actions. */
export interface PillActions {
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onMicrophone: () => void;
  onSystemAudio: () => void;
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
  systemAudio: boolean;
  actions: PillActions;
}

/** The running recording's controls. */
export function PillControls({ paused, systemAudio, actions }: PillControlsProps) {
  return (
    <span className="flex items-center gap-0.5">
      <Divider />
      <IconButton
        icon={paused ? Play : Pause}
        label={paused ? panelCopy.resume : panelCopy.pause}
        onClick={paused ? actions.onResume : actions.onPause}
      />
      <StopButton onStop={actions.onStop} />
      <IconButton icon={Mic} label={panelCopy.microphone} onClick={actions.onMicrophone} />
      <IconButton
        icon={systemAudio ? Volume2 : VolumeX}
        label={systemAudio ? panelCopy.systemAudioOn : panelCopy.systemAudioOff}
        aria-pressed={systemAudio}
        onClick={actions.onSystemAudio}
        className={cn(systemAudio ? 'text-accent' : 'text-fg-subtle')}
      />
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
