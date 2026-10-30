/**
 * Motion for the small cards that float at the screen's top-right edge (the
 * meeting reminder): each slides in from off-screen and back out when
 * answered, and only fades with Reduce Motion. Their windows reach the
 * screen's right edge so the slide starts beyond it. The exit hook is shared
 * with the top-centre cards (`ui/top-card`).
 */
import { useState } from 'react';

/** How long a card takes to slide out before its answer is sent. */
export const EDGE_EXIT_MS = 200;

/** The entrance and the exit, as Tailwind classes. */
export const EDGE_MOTION = {
  enter:
    'animate-in fade-in-0 duration-[380ms] ease-[cubic-bezier(0.22,1,0.36,1)] motion-safe:slide-in-from-right-[calc(100%+1rem)]',
  exit: 'animate-out fill-mode-forwards fade-out-0 duration-200 ease-in motion-safe:slide-out-to-right-[calc(100%+1rem)]',
} as const;

/** Plays the exit, then sends `answer`. */
export type EdgeLeave = (answer: () => void) => void;

/**
 * Whether the card is leaving, and `leave`: it plays the exit, then answers;
 * a second answer during it does nothing. A failed answer brings the card
 * back (with its error) and allows another.
 */
export function useEdgeExit(failed: boolean): [boolean, EdgeLeave] {
  const [leaving, setLeaving] = useState(false);
  const out = leaving && !failed;
  const leave: EdgeLeave = (answer) => {
    if (out) return;
    setLeaving(true);
    window.setTimeout(answer, EDGE_EXIT_MS);
  };
  return [out, leave];
}
