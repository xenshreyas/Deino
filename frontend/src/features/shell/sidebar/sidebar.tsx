/**
 * The app sidebar (ui.md §3 Sidebar, PARITY_PLAN §4.6): drag region and
 * actions, search, the nav registry, and pinned at the bottom the Settings
 * item and the record block. Resizable and collapsible with `[`. Importing a
 * recording lives in ⌘K, drag-and-drop and the Meetings header (UX-26).
 */
import { cn } from '@/ui/cn';

import { shellCopy } from '../copy';
import { useShellLayout } from '../layout-store';
import { navItemsIn } from '../nav';
import { RecordBlock } from './record-block';
import { SidebarHeader, SidebarSearch } from './sidebar-header';
import { SidebarResizeHandle } from './resize-handle';
import { NavLink, SidebarNav } from './sidebar-nav';

/** Settings and the record block. */
function SidebarFooter() {
  return (
    <div className="flex shrink-0 flex-col gap-px p-2">
      {navItemsIn('footer').map((item) => (
        <NavLink key={item.id} item={item} />
      ))}
      <div className="pt-2">
        <RecordBlock />
      </div>
    </div>
  );
}

/** The sidebar; collapsing slides it out and gives its width to the content. */
export function Sidebar() {
  const open = useShellLayout((state) => state.sidebarOpen);
  const width = useShellLayout((state) => state.sidebarWidth);
  return (
    <aside
      data-chrome
      aria-label={shellCopy.sidebarLabel}
      style={{ width: open ? width : 0 }}
      className={cn(
        'relative z-(--z-sidebar) flex h-full shrink-0 flex-col overflow-visible border-r bg-bg-subtle',
        'transition-[width,border-color] duration-(--dur-base) ease-out',
        open ? 'border-border' : 'border-transparent',
      )}
    >
      <div
        className={cn(
          'flex h-full flex-col overflow-hidden transition-[opacity,translate,visibility] duration-(--dur-base) ease-out',
          // `invisible` also takes the collapsed sidebar out of the tab order.
          open ? 'visible translate-x-0 opacity-100' : 'invisible -translate-x-3 opacity-0',
        )}
        style={{ width }}
      >
        <SidebarHeader />
        <SidebarSearch />
        <SidebarNav />
        <SidebarFooter />
      </div>
      {open ? <SidebarResizeHandle /> : null}
    </aside>
  );
}
