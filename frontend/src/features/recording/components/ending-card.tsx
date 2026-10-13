/**
 * Auto-stop's "Ending…" notice (CAP-07) as a top-right card, in the meeting
 * prompt's style (E2 of `docs/mockups/meeting-prompt.html`): a solid bar,
 * "Recording ends in 12s" over why, and Keep recording with the Deino mark,
 * split with a caret holding Stop now. It shares the prompt's window, which
 * Rust shows while the notice is up; the time left drains along the bottom.
 */
import { useState } from 'react';

import { BrandMark } from '@/ui/brand';
import { cn } from '@/ui/cn';
import { DrainBar } from '@/ui/drain-bar';
import { EDGE_MOTION, type EdgeLeave, useEdgeExit } from '@/ui/edge-card';
import { ChevronDown, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

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

/** Keep recording, joined to the caret with Stop now. */
function SplitButton({ props, leave }: { props: EndingCardViewProps; leave: EdgeLeave }) {
  return (
    <div className="flex h-10 shrink-0 overflow-hidden rounded-lg border border-border bg-bg shadow-xs">
      <button
        type="button"
        disabled={props.busy}
        onClick={() => {
          leave(props.onKeep);
        }}
        className="flex items-center gap-2 pr-3 pl-2.5 text-sm font-medium text-fg hover:bg-bg-muted disabled:opacity-50"
      >
        <BrandMark size={16} />
        {recordingCopy.keepRecording}
      </button>
      <DropdownMenu>
        <DropdownMenuTrigger
          disabled={props.busy}
          aria-label={recordingCopy.endingMore}
          className="flex w-8 items-center justify-center border-l border-border text-fg-muted hover:bg-bg-muted disabled:opacity-50"
        >
          <ChevronDown {...ICON_PROPS} className="size-3.5" />
        </DropdownMenuTrigger>
        <DropdownMenuContent side="bottom" align="end">
          <DropdownMenuItem
            onSelect={() => {
              leave(props.onStopNow);
            }}
          >
            {recordingCopy.stopNow}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

/** The card (presentational). */
export function EndingCardView(props: EndingCardViewProps) {
  const [leaving, leave] = useEdgeExit(false);
  const [drainMs] = useState(() => Math.max(0, Date.parse(props.ending.stopAt) - Date.now()));
  return (
    <section
      aria-label={recordingCopy.endingLabel}
      role="status"
      className={cn(
        'relative overflow-hidden rounded-xl bg-surface-raised text-fg shadow-popover',
        leaving ? EDGE_MOTION.exit : EDGE_MOTION.enter,
      )}
    >
      <div className="flex items-center gap-2.5 py-2.5 pr-2.5 pl-3">
        <span aria-hidden className="my-0.5 w-[3px] shrink-0 self-stretch rounded-full bg-brand" />
        <div className="min-w-0 flex-1 pr-1">
          <h1 className="truncate text-sm font-semibold text-fg tabular-nums">
            {recordingCopy.endsIn(props.seconds)}
          </h1>
          <p className="truncate text-xs text-fg-muted">{endingReason[props.ending.reason]}</p>
        </div>
        <SplitButton props={props} leave={leave} />
      </div>
      <DrainBar durationMs={drainMs} />
    </section>
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
