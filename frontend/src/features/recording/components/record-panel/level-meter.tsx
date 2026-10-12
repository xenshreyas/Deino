/**
 * The record panel's level meter (ui.md, Record pill): five 3px bars that
 * follow the real levels. The three middle bars are the microphone; the two
 * outer bars are the call (system audio) at 60%, so one meter shows both
 * sides. Heights are smoothed in the container; here they only map to
 * pixels. Reduced motion keeps the bars (they are information) but drops
 * the height transition.
 */
import { cn } from '@/ui/cn';

import type { AudioLevels } from '../../types';

/** What the bars show. */
export type MeterState = 'live' | 'paused' | 'arming' | 'warning';

/** Shortest and tallest bar, in pixels. */
const MIN_PX = 4;
const MAX_PX = 16;

/** Each bar's share of the level, and the source it follows. */
const BARS = [
  { reach: 0.6, source: 'system' },
  { reach: 0.85, source: 'microphone' },
  { reach: 1, source: 'microphone' },
  { reach: 0.75, source: 'microphone' },
  { reach: 0.55, source: 'system' },
] as const;

/**
 * A peak level as a bar's fill: speech peaks sit around 0.1–0.4, so the
 * square root spreads them over the bar's height.
 */
export function fill(level: number): number {
  return Math.min(1, Math.sqrt(Math.max(0, level)) * 1.6);
}

/** Props of {@link LevelMeter}. */
export interface LevelMeterProps {
  levels: AudioLevels;
  state: MeterState;
}

/** Five bars, decorative to assistive technology (the label says it all). */
export function LevelMeter({ levels, state }: LevelMeterProps) {
  return (
    <span aria-hidden className="flex h-4 items-center gap-0.5">
      {BARS.map((bar, index) => {
        const live = state === 'live' || state === 'warning';
        const height = live
          ? MIN_PX + fill(levels[bar.source]) * bar.reach * (MAX_PX - MIN_PX)
          : MIN_PX;
        return (
          <span
            key={index}
            style={{ height: `${String(height)}px` }}
            className={cn(
              'w-[3px] rounded-full transition-[height] duration-(--dur-instant) ease-linear motion-reduce:transition-none',
              bar.source === 'system' && 'opacity-60',
              state === 'live' && 'bg-accent',
              state === 'warning' && 'bg-warning',
              state === 'paused' && 'bg-fg-subtle',
              state === 'arming' && 'animate-level-bar bg-fg-subtle',
            )}
          />
        );
      })}
    </span>
  );
}
