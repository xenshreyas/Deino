/**
 * The scrubber: a 6px track painted with who spoke when (each speaker's
 * palette colour, faint ahead of the playhead and full behind it), a small
 * playhead, and a time tooltip under the pointer. It is a `slider`: drag or
 * click to seek, ←/→ move 5 s, PageUp/PageDown a minute, Home/End the ends.
 */
import { formatElapsed } from '@/features/recording';
import { type KeyboardEvent, type PointerEvent, useState } from 'react';

import { cn } from '@/ui/cn';

import { playerCopy } from '../copy';

/** Keyboard steps, in milliseconds. */
const STEP_MS = 5_000;
const PAGE_MS = 60_000;

/** Props of {@link Scrubber}. */
export interface ScrubberProps {
  /** The position, in milliseconds. */
  time: number;
  /** The length, in milliseconds; 0 while unknown. */
  duration: number;
  /** The speaker bands as a CSS gradient, or `null` without a transcript. */
  bands: string | null;
  onSeek: (ms: number) => void;
}

/** The position under the pointer, in milliseconds. */
function timeAt(event: PointerEvent<HTMLDivElement>, duration: number): number {
  const box = event.currentTarget.getBoundingClientRect();
  const fraction = box.width === 0 ? 0 : (event.clientX - box.left) / box.width;
  return Math.min(1, Math.max(0, fraction)) * duration;
}

/** Where a key moves the playhead from `time`, if it is a slider key. */
function keyTarget(key: string, time: number, duration: number): number | undefined {
  const moves: Record<string, number> = {
    ArrowLeft: time - STEP_MS,
    ArrowDown: time - STEP_MS,
    ArrowRight: time + STEP_MS,
    ArrowUp: time + STEP_MS,
    PageDown: time - PAGE_MS,
    PageUp: time + PAGE_MS,
    Home: 0,
    End: duration,
  };
  return moves[key];
}

/** Pointer seeking and the hover time. */
function usePointer(duration: number, onSeek: (ms: number) => void) {
  const [hover, setHover] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);
  return {
    hover,
    dragging,
    onPointerDown: (event: PointerEvent<HTMLDivElement>) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragging(true);
      onSeek(timeAt(event, duration));
    },
    onPointerMove: (event: PointerEvent<HTMLDivElement>) => {
      const at = timeAt(event, duration);
      setHover(at);
      if (dragging) onSeek(at);
    },
    onPointerUp: () => {
      setDragging(false);
    },
    onPointerLeave: () => {
      setHover(null);
    },
  };
}

/** The scrubber. */
export function Scrubber({ time, duration, bands, onSeek }: ScrubberProps) {
  const pointer = usePointer(duration, onSeek);
  const played = duration > 0 ? Math.min(100, (time / duration) * 100) : 0;
  const total = formatElapsed(duration / 1000);
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const target = keyTarget(event.key, time, duration);
    if (target === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    onSeek(Math.min(duration, Math.max(0, target)));
  };
  return (
    <div
      role="slider"
      tabIndex={0}
      aria-label={playerCopy.scrubber}
      aria-valuemin={0}
      aria-valuemax={Math.round(duration / 1000)}
      aria-valuenow={Math.round(time / 1000)}
      aria-valuetext={playerCopy.position(formatElapsed(time / 1000), total)}
      onKeyDown={onKeyDown}
      onPointerDown={pointer.onPointerDown}
      onPointerMove={pointer.onPointerMove}
      onPointerUp={pointer.onPointerUp}
      onPointerLeave={pointer.onPointerLeave}
      className="group/scrubber relative flex h-7 min-w-0 flex-1 cursor-pointer touch-none items-center rounded-sm outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring"
    >
      <Track played={played} bands={bands} />
      <Playhead played={played} active={pointer.dragging} />
      {pointer.hover === null || duration === 0 ? null : (
        <HoverTime at={pointer.hover} duration={duration} />
      )}
    </div>
  );
}

/** The track: speaker bands faint ahead of the playhead, full behind it. */
function Track({ played, bands }: { played: number; bands: string | null }) {
  const fill = bands ?? 'linear-gradient(var(--fg-muted), var(--fg-muted))';
  return (
    <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-bg-muted transition-[height] duration-(--dur-fast) ease-out group-hover/scrubber:h-2">
      <div className="absolute inset-0 opacity-35" style={{ backgroundImage: fill }} />
      <div
        className="absolute inset-0"
        style={{ backgroundImage: fill, clipPath: `inset(0 ${String(100 - played)}% 0 0)` }}
      />
    </div>
  );
}

/** The playhead: a small dot that grows while hovered, focused or dragged. */
function Playhead({ played, active }: { played: number; active: boolean }) {
  return (
    <span
      aria-hidden
      className={cn(
        'pointer-events-none absolute top-1/2 size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-fg shadow-[0_0_0_2px_var(--bg)]',
        'transition-transform duration-(--dur-fast) ease-out group-hover/scrubber:scale-125 group-focus-visible/scrubber:scale-125',
        active && 'scale-125',
      )}
      style={{ left: `${String(played)}%` }}
    />
  );
}

/** The time under the pointer, above the track. */
function HoverTime({ at, duration }: { at: number; duration: number }) {
  return (
    <span
      aria-hidden
      className="pointer-events-none absolute -top-6 -translate-x-1/2 rounded-sm bg-surface-raised px-1.5 py-0.5 text-xs text-fg tabular-nums shadow-popover"
      style={{ left: `${String((at / duration) * 100)}%` }}
    >
      {formatElapsed(at / 1000)}
    </span>
  );
}
