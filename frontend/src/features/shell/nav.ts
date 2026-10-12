/**
 * The sidebar's navigation registry (PARITY_PLAN §3 rule 4, §4.6).
 *
 * Append-only: a feature card adds its entry here (and its route file), and
 * the sidebar, the ⌘K palette's "Go to" group and the "coming soon" pages
 * all pick it up. An item with `comingSoon` shows a calm placeholder page
 * until the named card ships and removes the field.
 *
 * The sidebar stays short (UX-25): Home, Meetings, Action items, People and
 * the Folders tree, then a collapsed "More", then Settings. A page that is
 * reached from another page (Companies from People; Tags and Trash from
 * Meetings; Stats from Home) sets `sidebar: false`: it keeps its ⌘K entry.
 */
import type { ComponentType } from 'react';

import { FolderNavSection } from '@/features/folders';
import type { FileRoutesByTo } from '@/routeTree.gen';
import {
  Building2,
  ChartColumn,
  Folder,
  Home,
  type Icon,
  Lightbulb,
  ListChecks,
  NotebookText,
  ScrollText,
  Settings,
  Tag,
  Trash2,
  Users,
  Workflow,
} from '@/ui/icons';

/** A path of a route without parameters, checked against the route tree. */
export type NavPath = Exclude<keyof FileRoutesByTo, `${string}$${string}`>;

/** Where an item sits in the sidebar: the main list, the collapsed "More" group, or the footer. */
export type NavSection = 'primary' | 'more' | 'footer';

/** One sidebar entry. */
export interface NavItem {
  id: string;
  label: string;
  icon: Icon;
  to: NavPath;
  /** Sort key within its section; leave gaps of 10 for later items. */
  order: number;
  section: NavSection;
  /** `false` keeps the page out of the sidebar; ⌘K "Go to" still lists it. */
  sidebar?: false;
  /** Set while the feature is unbuilt: the card that builds it and one line on what it will do. */
  comingSoon?: { card: string; description: string };
  /**
   * Renders the entry instead of the plain link, for an entry that is more
   * than a link (the Folders tree). It must still link to `to`.
   */
  Component?: ComponentType;
}

/** Every destination, in sidebar order. */
export const NAV_ITEMS = [
  { id: 'home', label: 'Home', icon: Home, to: '/', order: 10, section: 'primary' },
  {
    id: 'meetings',
    label: 'Meetings',
    icon: NotebookText,
    to: '/meetings',
    order: 20,
    section: 'primary',
  },
  {
    id: 'action-items',
    label: 'Action items',
    icon: ListChecks,
    to: '/action-items',
    order: 30,
    section: 'primary',
  },
  { id: 'people', label: 'People', icon: Users, to: '/people', order: 40, section: 'primary' },
  {
    id: 'folders',
    label: 'Folders',
    icon: Folder,
    to: '/folders',
    order: 50,
    section: 'primary',
    // ORG-05: the entry is the folder tree (with Unfiled), not a plain link.
    Component: FolderNavSection,
  },
  // Reached from other pages (UX-25): a tab on People, the Meetings header, Home.
  {
    id: 'companies',
    label: 'Companies',
    icon: Building2,
    to: '/companies',
    order: 60,
    section: 'primary',
    sidebar: false,
  },
  {
    id: 'tags',
    label: 'Tags',
    icon: Tag,
    to: '/tags',
    order: 70,
    section: 'primary',
    sidebar: false,
  },
  {
    id: 'stats',
    label: 'Stats',
    icon: ChartColumn,
    to: '/stats',
    order: 80,
    section: 'primary',
    sidebar: false,
  },
  {
    id: 'trash',
    label: 'Trash',
    icon: Trash2,
    to: '/meetings/trash',
    order: 90,
    section: 'primary',
    sidebar: false,
  },
  {
    id: 'insights',
    label: 'Insights',
    icon: Lightbulb,
    to: '/insights',
    order: 10,
    section: 'more',
  },
  { id: 'recipes', label: 'Recipes', icon: ScrollText, to: '/recipes', order: 20, section: 'more' },
  {
    id: 'automations',
    label: 'Automations',
    icon: Workflow,
    to: '/automations',
    order: 30,
    section: 'more',
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: Settings,
    to: '/settings',
    order: 10,
    section: 'footer',
  },
] as const satisfies readonly NavItem[];

/** The ID of a sidebar entry. */
export type NavItemId = (typeof NAV_ITEMS)[number]['id'];

/** The sidebar items of one section, in order; `sidebar: false` items are left out. */
export function navItemsIn(section: NavSection): readonly NavItem[] {
  const items: readonly NavItem[] = NAV_ITEMS;
  return items
    .filter((item) => item.section === section && item.sidebar !== false)
    .sort((a, b) => a.order - b.order);
}

/** The item with `id`. */
export function navItem(id: NavItemId): NavItem {
  const item: NavItem | undefined = NAV_ITEMS.find((candidate) => candidate.id === id);
  if (item === undefined) throw new Error(`Unknown nav item: ${id}`);
  return item;
}

/** True when `pathname` is inside `item` (Home matches only `/`). */
export function isNavItemActive(item: NavItem, pathname: string): boolean {
  if (item.to === '/') return pathname === '/';
  return pathname === item.to || pathname.startsWith(`${item.to}/`);
}
