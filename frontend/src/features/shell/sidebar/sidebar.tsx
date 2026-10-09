/**
 * The app sidebar (ui.md §3 Sidebar, PARITY_PLAN §4.6): drag region and
 * actions, search, the nav registry, and pinned at the bottom the Settings
 * item and the record block. Resizable and collapsible with `[`.
 */
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { useConfig } from '@/contexts/ConfigContext';
import { cn } from '@/ui/cn';
import { ICON_PROPS, Upload } from '@/ui/icons';

import { shellCopy } from '../copy';
import { useShellLayout } from '../layout-store';
import { navItemsIn } from '../nav';
import { RecordBlock } from './record-block';
import { SidebarHeader, SidebarSearch } from './sidebar-header';
import { SidebarResizeHandle } from './resize-handle';
import { NavLink, navRowClassName, SidebarNav } from './sidebar-nav';

/** The "Import audio" row, shown while its beta feature is on. */
function ImportAudioRow() {
  const { betaFeatures } = useConfig();
  const { openImportDialog } = useImportDialog();
  if (!betaFeatures.importAndRetranscribe) return null;
  return (
    <button
      type="button"
      className={navRowClassName}
      onClick={() => {
        openImportDialog();
      }}
    >
      <Upload {...ICON_PROPS} />
      {shellCopy.importAudio}
    </button>
  );
}

/** Settings, the beta import row and the record block. */
function SidebarFooter() {
  return (
    <div className="flex shrink-0 flex-col gap-px px-2 pt-2 pb-3">
      <ImportAudioRow />
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
