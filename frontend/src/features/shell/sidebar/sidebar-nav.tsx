import { Link, useRouterState } from '@tanstack/react-router';
import { useState } from 'react';

import { cn } from '@/ui/cn';
import { ChevronRight, Ellipsis, ICON_PROPS } from '@/ui/icons';

import { shellCopy } from '../copy';
import { isNavItemActive, type NavItem, navItemsIn } from '../nav';

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

/** A registry entry: its own component (the Folders tree) or a plain link. */
function NavEntry({ item }: { item: NavItem }) {
  return item.Component === undefined ? <NavLink item={item} /> : <item.Component />;
}

/**
 * "More": the pages most people open rarely (Insights, Recipes,
 * Automations), closed by default and open while one of them is showing.
 */
function MoreGroup() {
  const items = navItemsIn('more');
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const [open, setOpen] = useState(false);
  const showing = open || items.some((item) => isNavItemActive(item, pathname));
  if (items.length === 0) return null;
  return (
    <div className="mt-2 flex flex-col gap-px">
      <button
        type="button"
        aria-expanded={showing}
        onClick={() => {
          setOpen(!showing);
        }}
        className={navRowClassName}
      >
        <Ellipsis {...ICON_PROPS} />
        <span className="flex-1 truncate text-left">{shellCopy.more}</span>
        <ChevronRight
          {...ICON_PROPS}
          size={14}
          aria-hidden
          className={cn('transition-transform duration-(--dur-fast)', showing && 'rotate-90')}
        />
      </button>
      {showing ? items.map((item) => <NavEntry key={item.id} item={item} />) : null}
    </div>
  );
}

/** The sidebar's scrolling navigation: the main entries, the Folders tree, then "More". */
export function SidebarNav() {
  return (
    <nav aria-label="Main" className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-3">
      <div className="flex flex-col gap-px">
        {navItemsIn('primary').map((item) => (
          <NavEntry key={item.id} item={item} />
        ))}
      </div>
      <MoreGroup />
    </nav>
  );
}
