import { type KeyboardEvent, type PointerEvent, useRef } from 'react';

import { shellCopy } from '../copy';
import { SIDEBAR_WIDTH, useShellLayout } from '../layout-store';

/** How far one arrow-key press moves the edge, in px. */
const KEYBOARD_STEP = 16;

/** The new width for an arrow, Home or End key, or `undefined` for other keys. */
export function widthForKey(key: string, width: number): number | undefined {
  if (key === 'ArrowLeft') return width - KEYBOARD_STEP;
  if (key === 'ArrowRight') return width + KEYBOARD_STEP;
  if (key === 'Home') return SIDEBAR_WIDTH.min;
  if (key === 'End') return SIDEBAR_WIDTH.max;
  return undefined;
}

/** Pointer and keyboard handlers that resize the sidebar. */
function useResizeHandlers() {
  const width = useShellLayout((state) => state.sidebarWidth);
  const setWidth = useShellLayout((state) => state.setSidebarWidth);
  const drag = useRef<{ startX: number; startWidth: number } | null>(null);
  const endDrag = () => {
    drag.current = null;
  };
  return {
    width,
    onPointerDown: (event: PointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      drag.current = { startX: event.clientX, startWidth: width };
    },
    onPointerMove: (event: PointerEvent<HTMLDivElement>) => {
      if (drag.current === null) return;
      setWidth(drag.current.startWidth + event.clientX - drag.current.startX);
    },
    onPointerUp: endDrag,
    onPointerCancel: endDrag,
    onDoubleClick: () => {
      setWidth(SIDEBAR_WIDTH.default);
    },
    onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => {
      const next = widthForKey(event.key, width);
      if (next === undefined) return;
      event.preventDefault();
      setWidth(next);
    },
  };
}

/**
 * The sidebar's right edge: drag (or use the arrow keys) to resize between
 * 200 and 320px; double-click to reset to 232px.
 */
export function SidebarResizeHandle() {
  const { width, ...handlers } = useResizeHandlers();
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={shellCopy.resizeSidebar}
      aria-valuemin={SIDEBAR_WIDTH.min}
      aria-valuemax={SIDEBAR_WIDTH.max}
      aria-valuenow={width}
      tabIndex={0}
      {...handlers}
      className="group absolute inset-y-0 -right-1 z-(--z-sidebar) w-2 cursor-col-resize focus-visible:outline-none"
    >
      <div className="mx-auto h-full w-px bg-transparent transition-colors duration-(--dur-fast) group-hover:bg-border-strong group-focus-visible:bg-focus-ring group-active:bg-accent" />
    </div>
  );
}
