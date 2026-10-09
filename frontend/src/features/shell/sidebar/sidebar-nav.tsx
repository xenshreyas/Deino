import { Link } from '@tanstack/react-router';

import { cn } from '@/ui/cn';
import { ICON_PROPS } from '@/ui/icons';

import { NAV_SECTION_LABELS, type NavItem, navItemsIn, type NavSection } from '../nav';

/** The nav row look (ui.md §3 Nav item): 30px, r6, muted idle, ink when active. */
export const navRowClassName = cn(
  'flex h-[30px] w-full items-center gap-2 rounded-sm px-2 text-sm text-fg-muted',
  'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg',
  'focus-visible:outline-offset-[-2px] data-[status=active]:bg-bg-muted data-[status=active]:text-fg',
);

/** One registry entry as a router link; the router marks it active. */
export function NavLink({ item }: { item: NavItem }) {
  const ItemIcon = item.icon;
  return (
    <Link to={item.to} activeOptions={{ exact: item.to === '/' }} className={navRowClassName}>
      <ItemIcon {...ICON_PROPS} />
      <span className="truncate">{item.label}</span>
    </Link>
  );
}

/** One section of the registry, with its label when it has one. */
function NavGroup({ section }: { section: NavSection }) {
  const label = NAV_SECTION_LABELS[section];
  return (
    <div className="flex flex-col gap-px">
      {label === undefined ? null : (
        <h2 className="mt-3 flex h-6 items-center px-2 text-xs font-semibold text-fg-muted">
          {label}
        </h2>
      )}
      {navItemsIn(section).map((item) => (
        <NavLink key={item.id} item={item} />
      ))}
    </div>
  );
}

/** The sidebar's scrolling navigation: every registry section except the footer. */
export function SidebarNav() {
  return (
    <nav aria-label="Main" className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-3">
      <NavGroup section="primary" />
      <NavGroup section="workspace" />
      <NavGroup section="intelligence" />
    </nav>
  );
}
