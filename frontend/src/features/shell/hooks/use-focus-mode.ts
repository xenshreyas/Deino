/**
 * Focus mode (NOT-01, Granola's fade): while the user types in a note, the
 * sidebar, title bar and other `data-chrome` regions fade out; moving the
 * pointer (or leaving the window) brings them back. The notepad turns it
 * on; this hook turns it off and returns the attribute the shell's root
 * carries (`data-focus-mode`), which `globals.css` styles.
 */
import { useMountEffect } from '@/hooks/useMountEffect';

import { useShellLayout } from '../layout-store';

/** Pointer travel (px) that counts as "the user reached for the mouse". */
const WAKE_DISTANCE = 4;

/** Ends focus mode on real pointer movement; returns whether it is on. */
export function useFocusMode(): boolean {
  const focusMode = useShellLayout((state) => state.focusMode);
  useMountEffect(() => {
    const wake = (event: PointerEvent) => {
      if (Math.abs(event.movementX) + Math.abs(event.movementY) < WAKE_DISTANCE) return;
      useShellLayout.getState().setFocusMode(false);
    };
    const leave = () => {
      useShellLayout.getState().setFocusMode(false);
    };
    window.addEventListener('pointermove', wake);
    window.addEventListener('blur', leave);
    return () => {
      window.removeEventListener('pointermove', wake);
      window.removeEventListener('blur', leave);
    };
  });
  return focusMode;
}
