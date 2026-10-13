/**
 * The meeting prompt card (presentational), design E2 of
 * `docs/mockups/meeting-prompt.html`, after Granola's: one row with a bar
 * (solid for a calendar meeting, dashed for a call that isn't on it), the
 * event (or "Meeting detected") over its time or app, and one split button,
 * Take notes with the Deino mark and a caret menu holding Ignore and
 * "Always ignore <app>". The auto-start countdown drains along the bottom.
 * Props in, events out; `DetectionPrompt` wires it to Rust.
 *
 * The card floats in its own transparent, non-activating window that
 * reaches the screen's right edge, so it slides in from off-screen and back
 * out when answered (a fade only with Reduce Motion).
 */
import { useState } from 'react';

import { BrandMark } from '@/ui/brand';
import { cn } from '@/ui/cn';
import { ChevronDown, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { detectionCopy } from '../copy';
import type { Detection } from '../types';
import { CountdownBar } from './countdown-bar';

/** How long the card takes to slide out before the answer goes to Rust. */
export const EXIT_MS = 200;

/** Props of {@link MeetingPrompt}. */
export interface MeetingPromptProps {
  detection: Detection;
  /** Seconds until recording starts on its own, or `null` without auto-start. */
  secondsLeft: number | null;
  /** Milliseconds until then, for the draining bar. */
  millisecondsLeft: number | null;
  /** An answer is on its way to Rust; the buttons wait. */
  busy: boolean;
  /** The last answer failed. */
  failed: boolean;
  onTakeNotes: () => void;
  onDismiss: () => void;
  onAlwaysIgnore: () => void;
}

/** Plays the exit, then sends `answer`. */
type Leave = (answer: () => void) => void;

/** The bar on the left: solid for a calendar meeting, dashed otherwise. */
function Bar({ scheduled }: { scheduled: boolean }) {
  return (
    <span
      aria-hidden
      className={cn(
        'my-0.5 w-[3px] shrink-0 self-stretch rounded-full',
        scheduled
          ? 'bg-brand'
          : 'bg-[repeating-linear-gradient(to_bottom,var(--brand)_0_4px,transparent_4px_7px)]',
      )}
    />
  );
}

/** The event (or "Meeting detected") over its time or app, and the countdown. */
function Heading({ detection, secondsLeft }: { detection: Detection; secondsLeft: number | null }) {
  const title = detection.event?.title ?? detectionCopy.titles[detection.kind];
  return (
    <div className="min-w-0 flex-1 pr-1">
      <h1 className="truncate text-sm font-semibold text-fg" title={title}>
        {title}
      </h1>
      <p className="flex min-w-0 gap-1 text-xs text-fg-muted">
        <span className="truncate">{detectionCopy.subtitle(detection)}</span>
        {secondsLeft === null ? null : (
          <span className="shrink-0 text-fg tabular-nums">
            {`· ${detectionCopy.countdown(secondsLeft)}`}
          </span>
        )}
      </p>
    </div>
  );
}

/** The caret's menu: Ignore (or Don't record) and Always ignore <app>. */
function IgnoreMenu({ props, leave }: { props: MeetingPromptProps; leave: Leave }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        disabled={props.busy}
        aria-label={detectionCopy.moreIgnore}
        className="flex w-8 items-center justify-center border-l border-border text-fg-muted hover:bg-bg-muted disabled:opacity-50"
      >
        <ChevronDown {...ICON_PROPS} className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent side="bottom" align="end">
        <DropdownMenuItem
          onSelect={() => {
            leave(props.onDismiss);
          }}
        >
          {props.secondsLeft === null ? detectionCopy.ignore : detectionCopy.dontRecord}
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            leave(props.onAlwaysIgnore);
          }}
        >
          {detectionCopy.alwaysIgnoreApp(props.detection.app.name)}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Take notes, carrying the Deino mark, joined to the caret menu. */
function SplitButton({ props, leave }: { props: MeetingPromptProps; leave: Leave }) {
  return (
    <div className="flex h-10 shrink-0 overflow-hidden rounded-lg border border-border bg-bg shadow-xs">
      <button
        type="button"
        disabled={props.busy}
        onClick={() => {
          leave(props.onTakeNotes);
        }}
        className="flex items-center gap-2 pr-3 pl-2.5 text-sm font-medium text-fg hover:bg-bg-muted disabled:opacity-50"
      >
        <BrandMark size={16} />
        {detectionCopy.takeNotes}
      </button>
      <IgnoreMenu props={props} leave={leave} />
    </div>
  );
}

/**
 * Plays the exit, then answers; a second answer during it does nothing. A
 * failed answer brings the card back (with its error) and allows another.
 */
function useLeave(failed: boolean): [boolean, Leave] {
  const [leaving, setLeaving] = useState(false);
  const out = leaving && !failed;
  const leave: Leave = (answer) => {
    if (out) return;
    setLeaving(true);
    window.setTimeout(answer, EXIT_MS);
  };
  return [out, leave];
}

/** The slide in from off-screen right, and back out. */
const MOTION = {
  enter:
    'animate-in fade-in-0 duration-[380ms] ease-[cubic-bezier(0.22,1,0.36,1)] motion-safe:slide-in-from-right-[calc(100%+1rem)]',
  exit: 'animate-out fill-mode-forwards fade-out-0 duration-200 ease-in motion-safe:slide-out-to-right-[calc(100%+1rem)]',
} as const;

/** The meeting prompt card. */
export function MeetingPrompt(props: MeetingPromptProps) {
  const { detection, secondsLeft, millisecondsLeft } = props;
  const [leaving, leave] = useLeave(props.failed);
  return (
    <section
      aria-label={detectionCopy.promptLabel}
      className={cn(
        'relative overflow-hidden rounded-xl bg-surface-raised text-fg shadow-popover',
        leaving ? MOTION.exit : MOTION.enter,
      )}
    >
      <div className="flex items-center gap-2.5 py-2.5 pr-2.5 pl-3">
        <Bar scheduled={detection.event !== null} />
        <Heading detection={detection} secondsLeft={secondsLeft} />
        <SplitButton props={props} leave={leave} />
      </div>
      {props.failed ? (
        <p role="alert" className="px-3 pb-2 text-xs text-danger">
          {detectionCopy.failed}
        </p>
      ) : null}
      {millisecondsLeft === null ? null : <CountdownBar durationMs={millisecondsLeft} />}
    </section>
  );
}
