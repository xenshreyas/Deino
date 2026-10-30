/**
 * The record pill (ui.md, Record pill; Granola's nub). Collapsed it is a
 * pulsing dot, the level meter and the time; on hover it grows away from
 * the screen edge to show the controls. A health issue turns the meter
 * orange and names the problem; auto-stop's countdown replaces the
 * controls with "Keep recording". Presentational: the container wires it.
 *
 * The width morphs with a grid track (`0fr` → `1fr`) instead of measured
 * widths, so the pill never jumps and the window itself never resizes.
 */
import type { PointerEventHandler } from 'react';

import { cn } from '@/ui/cn';
import { LoaderCircle } from '@/ui/icons';

import { panelCopy } from '../../copy';
import type { AudioLevels, RecordingPhase } from '../../types';
import { LevelMeter, type MeterState } from './level-meter';
import { type PillActions, PillControls } from './pill-controls';

/** What the pill shows. */
export interface PillView {
  phase: RecordingPhase;
  /** `12:04`. */
  elapsed: string;
  levels: AudioLevels;
  /** The pointer is over the pill (or it has keyboard focus). */
  expanded: boolean;
  /** The first open health issue, short, if any. */
  warning: string | null;
}

/** Props of {@link Pill}. */
export interface PillProps {
  view: PillView;
  actions: PillActions;
  /** Press-to-open and drag-to-move, on the pill's body. */
  onPointerDown: PointerEventHandler<HTMLDivElement>;
}

function meterState(view: PillView): MeterState {
  if (view.phase === 'starting' || view.phase === 'stopping') return 'arming';
  if (view.phase === 'paused') return 'paused';
  return view.warning === null ? 'live' : 'warning';
}

/** The red dot: pulsing while capturing, still and grey while paused. */
function RecordDot({ paused }: { paused: boolean }) {
  return (
    <span
      aria-hidden
      className={cn(
        'size-[7px] shrink-0 rounded-full',
        paused ? 'bg-fg-subtle' : 'animate-record-dot bg-recording motion-reduce:animate-none',
      )}
    />
  );
}

/** The time, the warning, or the phase while starting and saving. */
function Status({ view }: { view: PillView }) {
  if (view.phase === 'starting' || view.phase === 'stopping') {
    return (
      <span className="flex items-center gap-1.5 text-xs text-fg-muted">
        {view.phase === 'stopping' ? (
          <LoaderCircle size={12} className="animate-spin" aria-hidden />
        ) : null}
        {view.phase === 'starting' ? panelCopy.starting : panelCopy.saving}
      </span>
    );
  }
  return (
    <span className="flex items-center gap-2 text-xs font-semibold tabular-nums">
      <span className={view.phase === 'paused' ? 'text-fg-muted' : 'text-fg'}>{view.elapsed}</span>
      {view.warning === null ? null : (
        <span role="alert" className="font-normal whitespace-nowrap text-warning-text">
          {view.warning}
        </span>
      )}
    </span>
  );
}

/** The part that grows on hover (auto-stop's notice is a top-centre card). */
function Reveal({ view, actions }: { view: PillView; actions: PillActions }) {
  const running = view.phase === 'recording' || view.phase === 'paused';
  const open = running && view.expanded;
  return (
    <span
      className={cn(
        'grid transition-[grid-template-columns,opacity] duration-(--dur-pill) ease-out motion-reduce:transition-none',
        open ? 'grid-cols-[1fr] opacity-100' : 'grid-cols-[0fr] opacity-0',
      )}
    >
      <span className="min-w-0 overflow-hidden">
        {open ? <PillControls paused={view.phase === 'paused'} actions={actions} /> : null}
      </span>
    </span>
  );
}

/** The record pill. */
export function Pill({ view, actions, onPointerDown }: PillProps) {
  const tall = view.expanded;
  return (
    <div
      role="group"
      aria-label={panelCopy.label}
      onPointerDown={onPointerDown}
      className={cn(
        'flex w-max cursor-default items-center gap-2 rounded-full pl-3 select-none',
        'bg-surface-raised/92 shadow-pill backdrop-blur-xl backdrop-saturate-150',
        'transition-[height,box-shadow] duration-(--dur-pill) ease-out motion-reduce:transition-none',
        tall ? 'h-11 pr-1.5' : 'h-9 pr-3',
        view.phase === 'paused' && 'ring-1 ring-warning/50',
      )}
    >
      <RecordDot paused={view.phase !== 'recording'} />
      <LevelMeter levels={view.levels} state={meterState(view)} />
      <Status view={view} />
      <Reveal view={view} actions={actions} />
    </div>
  );
}
