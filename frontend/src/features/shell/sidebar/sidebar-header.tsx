import { useNavigate } from '@tanstack/react-router';

import { BrandMark } from '@/ui/brand';
import { Kbd } from '@/ui/kbd';
import { IconButton } from '@/ui/icon-button';
import { ICON_PROPS, PanelLeft, Search, SquarePen } from '@/ui/icons';

import { shellCopy } from '../copy';
import { useShellLayout } from '../layout-store';
import { SHORTCUTS } from '../shortcuts';

/**
 * The sidebar's top: a 44px drag region with room for the traffic lights,
 * the small brand mark, then the sidebar toggle and New note (ui.md §3
 * Sidebar). The mark lets clicks through, so it drags the window too.
 */
export function SidebarHeader() {
  const toggleSidebar = useShellLayout((state) => state.toggleSidebar);
  const navigate = useNavigate();
  return (
    <div data-tauri-drag-region className="flex h-11 shrink-0 items-center gap-0.5 pr-2 pl-20">
      <BrandMark className="pointer-events-none mr-auto ml-1" />
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
    </div>
  );
}

/** The search row: looks like a field, opens the ⌘K palette (as in Granola). */
export function SidebarSearch() {
  const setPaletteOpen = useShellLayout((state) => state.setPaletteOpen);
  return (
    <div className="shrink-0 px-2 pb-2">
      <button
        type="button"
        onClick={() => {
          setPaletteOpen(true);
        }}
        className="flex h-[30px] w-full items-center gap-2 rounded-sm bg-bg-muted px-2 text-sm text-fg-subtle transition-colors duration-(--dur-fast) hover:text-fg-muted"
      >
        <Search {...ICON_PROPS} />
        <span className="flex-1 text-left">{shellCopy.search}</span>
        <Kbd>{SHORTCUTS.palette.display}</Kbd>
      </button>
    </div>
  );
}
