/**
 * A 2px brand line along a floating card's bottom edge that drains over the
 * time the card has left (the meeting prompt's 30 s or its auto-start, the
 * reminder's minute), as Granola's pop-ups do. It runs once, from the
 * duration it mounts with; with reduced motion it stays still.
 */
import { useCallback, useRef } from 'react';

import { cn } from '@/ui/cn';

/** Whether the user asked for reduced motion. */
function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Props of {@link DrainBar}. */
export interface DrainBarProps {
  /** Milliseconds until the card's moment: it closes, or recording starts. */
  durationMs: number;
  /** Height and colour; a 2px brand line by default. */
  className?: string | undefined;
}

/** The draining countdown line. */
export function DrainBar({ durationMs, className }: DrainBarProps) {
  const duration = useRef(durationMs);
  const drain = useCallback((bar: HTMLSpanElement | null) => {
    if (bar === null || typeof bar.animate !== 'function' || prefersReducedMotion()) return;
    bar.animate([{ transform: 'scaleX(1)' }, { transform: 'scaleX(0)' }], {
      duration: duration.current,
      easing: 'linear',
      fill: 'forwards',
    });
  }, []);
  return (
    <span
      ref={drain}
      aria-hidden="true"
      className={cn('absolute inset-x-0 bottom-0 origin-left', className ?? 'h-0.5 bg-brand')}
    />
  );
}
