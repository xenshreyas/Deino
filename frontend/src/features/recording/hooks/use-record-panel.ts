/**
 * The record panel's window-side state (CAP-10): hover (Rust watches the
 * pointer, since the window lets clicks through outside the pill), the
 * live levels, the pill's rectangle and the edge it hugs, press-or-drag on
 * its body, the theme, and auto-stop's countdown.
 */
import { type PointerEvent as ReactPointerEvent, type RefObject, useRef, useState } from 'react';

import { applyTheme, parseThemePreference, THEME_STORAGE_KEY } from '@/features/appearance';
import { useMountEffect } from '@/hooks/useMountEffect';
import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { recordingApi } from '../api';
import { dragPanel } from '../panel-window';
import type { AudioLevels, PanelEdge } from '../types';

/** How long the pill stays open after the pointer leaves (ui.md: 400 ms). */
const COLLAPSE_DELAY_MS = 400;

/** How far the pointer moves before a press becomes a drag. */
const DRAG_SLOP_PX = 4;

/** Level smoothing: `0.7·previous + 0.3·new` (Handy). */
const KEEP = 0.7;

/** Silence. */
export const QUIET: AudioLevels = { microphone: 0, system: 0 };

/** Whether the pill is open: the pointer is over it, after a short linger. */
export function usePanelHover() {
  const [inside, setInside] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  const set = (next: boolean) => {
    window.clearTimeout(timer.current);
    if (next) {
      setInside(true);
      return;
    }
    timer.current = window.setTimeout(() => {
      setInside(false);
    }, COLLAPSE_DELAY_MS);
  };
  useIpcEvent(events.panelPointer, ({ inside: over }) => {
    set(over);
  });
  return {
    expanded: inside,
    onPointerEnter: () => {
      set(true);
    },
    onPointerLeave: () => {
      set(false);
    },
  };
}

/** The levels, smoothed, about 30 times a second while capturing. */
export function usePanelLevels(): AudioLevels {
  const [levels, setLevels] = useState(QUIET);
  useIpcEvent(events.audioLevel, ({ levels: next }) => {
    setLevels((previous) => ({
      microphone: KEEP * previous.microphone + (1 - KEEP) * next.microphone,
      system: KEEP * previous.system + (1 - KEEP) * next.system,
    }));
  });
  return levels;
}

/**
 * Tells Rust where the pill (and the card under it) is whenever it
 * resizes, and returns the edge the panel hugs.
 */
export function usePanelRegion(stack: RefObject<HTMLElement | null>): PanelEdge {
  const [edge, setEdge] = useState<PanelEdge>('right');
  useIpcEvent(events.panelSide, ({ edge: next }) => {
    setEdge(next);
  });
  useMountEffect(() => {
    const element = stack.current;
    if (element === null) return undefined;
    const report = () => {
      const { x, y, width, height } = element.getBoundingClientRect();
      recordingApi.setPanelRegion({ x, y, width, height }).then(setEdge, (error: unknown) => {
        console.error('[recording] Could not report the pill', error);
      });
    };
    const observer = new ResizeObserver(report);
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  });
  return edge;
}

/** A press on the pill's body opens the notes; a press that moves drags the panel. */
export function usePressOrDrag(onPress: () => void) {
  return (start: ReactPointerEvent<HTMLElement>) => {
    const onControl = start.target instanceof Element && start.target.closest('button') !== null;
    if (start.button !== 0 || onControl) return;
    const { clientX, clientY } = start;
    let dragged = false;
    const move = (event: PointerEvent) => {
      if (dragged || Math.hypot(event.clientX - clientX, event.clientY - clientY) < DRAG_SLOP_PX)
        return;
      dragged = true;
      dragPanel();
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      if (!dragged) onPress();
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up, { once: true });
  };
}

/** Follows the theme the main window saves (the `storage` event reaches every window). */
export function usePanelTheme(): void {
  useMountEffect(() => {
    const apply = () => {
      applyTheme(parseThemePreference(window.localStorage.getItem(THEME_STORAGE_KEY)));
    };
    const changed = (event: StorageEvent) => {
      if (event.key === THEME_STORAGE_KEY) apply();
    };
    apply();
    window.addEventListener('storage', changed);
    return () => {
      window.removeEventListener('storage', changed);
    };
  });
}

/** Whole seconds until `at` (ISO), ticking; `null` without a deadline. */
export function useSecondsUntil(at: string | null): number | null {
  const [now, setNow] = useState(Date.now);
  useMountEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 250);
    return () => {
      window.clearInterval(timer);
    };
  });
  if (at === null) return null;
  return Math.max(0, Math.ceil((Date.parse(at) - now) / 1000));
}
