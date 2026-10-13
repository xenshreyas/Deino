/**
 * A 2px brand line along the prompt's bottom edge that drains to the
 * auto-start moment (ui.md: progress is a 2px accent bar). It runs once,
 * from the duration it mounts with; with reduced motion it stays still and
 * the seconds label carries the countdown alone.
 */
import { useCallback, useRef } from 'react';

/** Whether the user asked for reduced motion. */
function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Props of {@link CountdownBar}. */
export interface CountdownBarProps {
  /** Milliseconds until recording starts. */
  durationMs: number;
}

/** The draining countdown line. */
export function CountdownBar({ durationMs }: CountdownBarProps) {
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
      className="absolute inset-x-0 bottom-0 h-0.5 origin-left bg-brand"
    />
  );
}
