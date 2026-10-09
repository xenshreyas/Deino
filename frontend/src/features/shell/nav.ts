/**
 * The sidebar's navigation registry (PARITY_PLAN §3 rule 4, §4.6).
 *
 * Append-only: a feature card adds its entry here (and its route file), and
 * the sidebar, the ⌘K palette's "Go to" group and the "coming soon" pages
 * all pick it up. An item with `comingSoon` shows a calm placeholder page
 * until the named card ships and removes the field.
 */
import type { FileRoutesByTo } from '@/routeTree.gen';
import {
  Building2,
  Folder,
  Home,
  type Icon,
  Lightbulb,
  ListChecks,
  NotebookText,
  ScrollText,
  Settings,
  Tag,
  Users,
  Workflow,
} from '@/ui/icons';

/** A path of a route without parameters, checked against the route tree. */
export type NavPath = Exclude<keyof FileRoutesByTo, `${string}$${string}`>;

/** Where an item sits in the sidebar. */
export type NavSection = 'primary' | 'workspace' | 'intelligence' | 'footer';

/** One sidebar entry. */
export interface NavItem {
  id: string;
  label: string;
  icon: Icon;
  to: NavPath;
  /** Sort key within its section; leave gaps of 10 for later items. */
  order: number;
  section: NavSection;
  /** Set while the feature is unbuilt: the card that builds it and one line on what it will do. */
  comingSoon?: { card: string; description: string };
}

/** Section labels; `primary` and `footer` have none. */
export const NAV_SECTION_LABELS: Record<NavSection, string | undefined> = {
  primary: undefined,
  workspace: 'Workspace',
  intelligence: 'Intelligence',
  footer: undefined,
};

/** Every sidebar entry. */
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
    label: 'Action Items',
    icon: ListChecks,
    to: '/action-items',
    order: 30,
    section: 'primary',
    comingSoon: {
      card: 'AI-05',
      description: 'Every follow-up from your meetings, with owners and due dates.',
    },
  },
  {
    id: 'people',
    label: 'People',
    icon: Users,
    to: '/people',
    order: 10,
    section: 'workspace',
    comingSoon: {
      card: 'ORG-04',
      description: 'Everyone you meet, with what was said and what is still open.',
    },
  },
  {
    id: 'companies',
    label: 'Companies',
    icon: Building2,
    to: '/companies',
    order: 20,
    section: 'workspace',
    comingSoon: {
      card: 'ORG-04',
      description: 'The organisations behind your meetings, in one place.',
    },
  },
  {
    id: 'folders',
    label: 'Folders',
    icon: Folder,
    to: '/folders',
    order: 30,
    section: 'workspace',
    comingSoon: { card: 'ORG-05', description: 'Group meetings by project, client or team.' },
  },
  {
    id: 'tags',
    label: 'Tags',
    icon: Tag,
    to: '/tags',
    order: 40,
    section: 'workspace',
    comingSoon: {
      card: 'ORG-06',
      description: 'Label meetings by hand or let Deino tag them for you.',
    },
  },
  {
    id: 'insights',
    label: 'Insights',
    icon: Lightbulb,
    to: '/insights',
    order: 10,
    section: 'intelligence',
    comingSoon: {
      card: 'AI-06',
      description: 'Track the questions you care about across every meeting.',
    },
  },
  {
    id: 'recipes',
    label: 'Recipes',
    icon: ScrollText,
    to: '/recipes',
    order: 20,
    section: 'intelligence',
    comingSoon: {
      card: 'AI-04',
      description: 'Reusable prompts for follow-ups, briefs and summaries.',
    },
  },
  {
    id: 'automations',
    label: 'Automations',
    icon: Workflow,
    to: '/automations',
    order: 30,
    section: 'intelligence',
    comingSoon: {
      card: 'AUT-01',
      description: 'Send notes and tasks where they belong after each meeting.',
    },
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

/** The items of one section, in order. */
export function navItemsIn(section: NavSection): readonly NavItem[] {
  return NAV_ITEMS.filter((item) => item.section === section).sort((a, b) => a.order - b.order);
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
