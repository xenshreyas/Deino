/**
 * The meeting prompt card (presentational): what was detected, in which app,
 * the calendar event it belongs to, the auto-start countdown, and the three
 * answers. Props in, events out; `DetectionPrompt` wires it to Rust.
 *
 * The card floats in its own transparent, non-activating window, so it
 * carries the floating-layer recipe (raised surface, r12, popover shadow)
 * and enters like a popover from the top edge it is pinned to.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { Headphones, type Icon, ICON_PROPS, Phone, Video } from '@/ui/icons';

import { detectionCopy } from '../copy';
import type { Detection, MeetingKind } from '../types';
import { CountdownBar } from './countdown-bar';

/** The icon for each kind of call. */
const KIND_ICONS: Record<MeetingKind, Icon> = { meeting: Video, huddle: Headphones, call: Phone };

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

/** The kind's icon on the accent tint, with a live dot: the microphone is in use. */
function KindMark({ kind }: { kind: MeetingKind }) {
  const KindIcon = KIND_ICONS[kind];
  return (
    <span className="relative flex size-8 shrink-0 items-center justify-center rounded-md bg-accent-soft text-accent">
      <KindIcon {...ICON_PROPS} />
      <span className="absolute -top-0.5 -right-0.5 size-2 animate-record-dot rounded-full bg-accent ring-2 ring-surface-raised" />
    </span>
  );
}

/**
 * Title, then the event (if linked), then the app with the countdown beside
 * it: the countdown never truncates, the app name gives way first.
 */
function Heading({ detection, secondsLeft }: { detection: Detection; secondsLeft: number | null }) {
  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <h1 className="text-sm font-semibold text-fg">{detectionCopy.titles[detection.kind]}</h1>
      {detection.event === null ? null : (
        <p className="truncate text-sm text-fg" title={detection.event.title}>
          {detection.event.title}
        </p>
      )}
      <p className="flex min-w-0 gap-1 text-xs text-fg-muted">
        <span className="truncate">{detection.app.name}</span>
        {secondsLeft === null ? null : (
          <span className="shrink-0 text-fg tabular-nums">
            {`· ${detectionCopy.countdown(secondsLeft)}`}
          </span>
        )}
      </p>
    </div>
  );
}

/** The three answers: the quiet permanent one apart, the two everyday ones together. */
function Actions({ props }: { props: MeetingPromptProps }) {
  const { busy } = props;
  return (
    <footer className="flex items-center gap-1 border-t border-border px-2 py-2">
      <Button
        variant="ghost"
        size="sm"
        className="px-2 text-xs"
        disabled={busy}
        onClick={props.onAlwaysIgnore}
      >
        {detectionCopy.alwaysIgnore}
      </Button>
      <span className="flex-1" />
      <Button variant="ghost" size="sm" disabled={busy} onClick={props.onDismiss}>
        {props.secondsLeft === null ? detectionCopy.ignore : detectionCopy.dontRecord}
      </Button>
      <Button variant="primary" size="sm" disabled={busy} onClick={props.onTakeNotes}>
        {detectionCopy.takeNotes}
      </Button>
    </footer>
  );
}

/** The meeting prompt card. */
export function MeetingPrompt(props: MeetingPromptProps) {
  const { detection, secondsLeft, millisecondsLeft } = props;
  return (
    <section
      aria-label={detectionCopy.promptLabel}
      className={cn(
        'relative flex flex-col overflow-hidden rounded-lg bg-surface-raised text-fg shadow-popover',
        'animate-in duration-(--dur-slow) ease-out fade-in-0 slide-in-from-top-1',
      )}
    >
      <div className="flex items-start gap-3 px-4 pt-3.5 pb-3">
        <KindMark kind={detection.kind} />
        <Heading detection={detection} secondsLeft={secondsLeft} />
      </div>
      {props.failed ? (
        <p role="alert" className="px-4 pb-2 text-xs text-danger">
          {detectionCopy.failed}
        </p>
      ) : null}
      <Actions props={props} />
      {millisecondsLeft === null ? null : <CountdownBar durationMs={millisecondsLeft} />}
    </section>
  );
}
