/**
 * The content column's 44px title bar: a window drag region (the macOS title
 * bar is transparent, `titleBarStyle: "Overlay"`) that holds the sidebar
 * toggle while the sidebar is collapsed, and whatever actions the current
 * page portals into it with {@link TitleBarActions}.
 */
import { useNavigate } from '@tanstack/react-router';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';

import { cn } from '@/ui/cn';
import { IconButton } from '@/ui/icon-button';
import { PanelLeft, SquarePen } from '@/ui/icons';

import { shellCopy } from '../copy';
import { useShellLayout } from '../layout-store';
import { SHORTCUTS } from '../shortcuts';

/** The title bar; the sidebar's buttons move here while it is collapsed. */
export function TitleBar() {
  const sidebarOpen = useShellLayout((state) => state.sidebarOpen);
  const toggleSidebar = useShellLayout((state) => state.toggleSidebar);
  const setTitleBarSlot = useShellLayout((state) => state.setTitleBarSlot);
  const navigate = useNavigate();
  return (
    <div
      data-tauri-drag-region
      data-chrome
      className={cn(
        'flex h-11 shrink-0 items-center gap-0.5 pr-3 transition-[padding] duration-(--dur-base) ease-out',
        sidebarOpen ? 'pl-3' : 'pl-20',
      )}
    >
      {sidebarOpen ? null : (
        <>
          <IconButton
            icon={PanelLeft}
            label={shellCopy.toggleSidebar}
            shortcut={SHORTCUTS.toggleSidebar.display}
            onClick={toggleSidebar}
          />
          <IconButton
            icon={SquarePen}
            label={shellCopy.newNote}
            shortcut={SHORTCUTS.newNote.display}
            onClick={() => void navigate({ to: '/' })}
          />
        </>
      )}
      <div
        ref={setTitleBarSlot}
        data-tauri-drag-region
        className="flex h-full min-w-0 flex-1 items-center gap-1"
      />
    </div>
  );
}

/** Renders `children` in the title bar while the calling page is mounted. */
export function TitleBarActions({ children }: { children: ReactNode }) {
  const slot = useShellLayout((state) => state.titleBarSlot);
  return slot === null ? null : createPortal(children, slot);
}
