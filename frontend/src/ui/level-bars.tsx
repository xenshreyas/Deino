/**
 * The five-bar level meter of the recording state (ui.md §3 Sidebar record
 * block, Record pill): 3px bars, 2px apart, up to 16px tall, in the accent.
 *
 * Until CAP-10 streams real microphone levels, `live` bars move on a gentle
 * staggered wave; reduced motion holds them still.
 */
import { cn } from './cn';

/** What the meter shows. */
export type LevelBarsState = 'live' | 'paused' | 'idle';

/** Per-bar resting heights and wave offsets, so the five bars never move in step. */
const BARS = [
  { height: 'h-2', delay: '[animation-delay:-0.45s]' },
  { height: 'h-3.5', delay: '[animation-delay:-0.1s]' },
  { height: 'h-4', delay: '[animation-delay:-0.7s]' },
  { height: 'h-3', delay: '[animation-delay:-0.3s]' },
  { height: 'h-2.5', delay: '[animation-delay:-0.6s]' },
] as const;

/** Props of {@link LevelBars}. */
export interface LevelBarsProps {
  state: LevelBarsState;
  className?: string;
}

/** Five small bars; decorative, so hidden from assistive technology. */
export function LevelBars({ state, className }: LevelBarsProps) {
  return (
    <span aria-hidden className={cn('flex h-4 items-center gap-0.5', className)}>
      {BARS.map((bar, index) => (
        <span
          key={index}
          className={cn(
            'w-[3px] origin-center rounded-full',
            state === 'live' ? [bar.height, bar.delay, 'animate-level-bar bg-accent'] : 'h-1',
            state === 'paused' && 'bg-fg-subtle',
            state === 'idle' && 'bg-border-strong',
          )}
        />
      ))}
    </span>
  );
}
