/**
 * The meeting prompt card (presentational): a top-centre card, after
 * Notion's meeting prompt, with the event (or "Meeting detected") over its
 * time or app, Take notes and Ignore (its caret holds "Always ignore
 * <app>"), and a close button on hover. The time it has left (until it
 * closes, or until auto-start) drains along the bottom.
 * Props in, events out; `DetectionPrompt` wires it to Rust.
 *
 * The card floats in its own transparent, non-activating window centred
 * under the menu bar; it slides down on arrival and back up when answered.
 */
import { ChevronDown, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { TOP_BUTTON, TopCard, TopCardHeading, useTopExit } from '@/ui/top-card';

import { detectionCopy } from '../copy';
import type { Detection } from '../types';

/** Props of {@link MeetingPrompt}. */
export interface MeetingPromptProps {
  detection: Detection;
  /** Seconds until recording starts on its own, or `null` without auto-start. */
  secondsLeft: number | null;
  /** Milliseconds until the prompt closes or recording starts, for the line. */
  millisecondsLeft: number;
  /** An answer is on its way to Rust; the buttons wait. */
  busy: boolean;
  /** The last answer failed. */
  failed: boolean;
  onTakeNotes: () => void;
  onDismiss: () => void;
  onAlwaysIgnore: () => void;
}

/** The time or app, and the auto-start countdown when there is one. */
function Detail({ detection, secondsLeft }: { detection: Detection; secondsLeft: number | null }) {
  return (
    <>
      <span className="truncate">{detectionCopy.subtitle(detection)}</span>
      {secondsLeft === null ? null : (
        <span className="shrink-0 text-fg tabular-nums">{`· ${detectionCopy.countdown(secondsLeft)}`}</span>
      )}
    </>
  );
}

/** Ignore (or Don't record), with a caret holding "Always ignore <app>". */
function IgnoreButton({
  props,
  leave,
}: {
  props: MeetingPromptProps;
  leave: (answer: () => void) => void;
}) {
  return (
    <div className="flex flex-1 overflow-hidden rounded-lg">
      <button
        type="button"
        disabled={props.busy}
        onClick={() => {
          leave(props.onDismiss);
        }}
        className={`${TOP_BUTTON.quiet} rounded-r-none`}
      >
        {props.secondsLeft === null ? detectionCopy.ignore : detectionCopy.dontRecord}
      </button>
      <DropdownMenu>
        <DropdownMenuTrigger
          disabled={props.busy}
          aria-label={detectionCopy.moreIgnore}
          className="flex w-7 items-center justify-center border-l border-border bg-bg-muted text-fg-muted hover:brightness-95 disabled:opacity-50"
        >
          <ChevronDown {...ICON_PROPS} className="size-3.5" />
        </DropdownMenuTrigger>
        <DropdownMenuContent side="bottom" align="end">
          <DropdownMenuItem
            onSelect={() => {
              leave(props.onAlwaysIgnore);
            }}
          >
            {detectionCopy.alwaysIgnoreApp(props.detection.app.name)}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

/** The meeting prompt card. */
export function MeetingPrompt(props: MeetingPromptProps) {
  const { detection, secondsLeft, millisecondsLeft } = props;
  const [leaving, leave] = useTopExit(props.failed);
  const title = detection.event?.title ?? detectionCopy.titles[detection.kind];
  return (
    <TopCard
      label={detectionCopy.promptLabel}
      leaving={leaving}
      drainMs={millisecondsLeft}
      closeLabel={detectionCopy.close}
      onClose={() => {
        leave(props.onDismiss);
      }}
    >
      <TopCardHeading
        title={title}
        detail={<Detail detection={detection} secondsLeft={secondsLeft} />}
      />
      <div className="mt-3 flex gap-2">
        <button
          type="button"
          disabled={props.busy}
          onClick={() => {
            leave(props.onTakeNotes);
          }}
          className={TOP_BUTTON.primary}
        >
          {detectionCopy.takeNotes}
        </button>
        <IgnoreButton props={props} leave={leave} />
      </div>
      {props.failed ? (
        <p role="alert" className="mt-2 text-xs text-danger">
          {detectionCopy.failed}
        </p>
      ) : null}
    </TopCard>
  );
}
