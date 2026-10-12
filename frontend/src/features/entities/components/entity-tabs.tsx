/**
 * People and Companies as two tabs of one page (UX-25): Companies left the
 * sidebar, so the People page links to it here, and back. Router links, not
 * Radix tabs, so each list keeps its own URL.
 */
import { Link } from '@tanstack/react-router';

import { cn } from '@/ui/cn';

import { entitiesCopy } from '../copy';

/** The underline-tab look of `ui/tabs.tsx`, keyed on the router's active state. */
const TAB_CLASS = cn(
  'relative -mb-px inline-flex items-center text-sm whitespace-nowrap text-fg-muted',
  'transition-colors duration-(--dur-fast) ease-out hover:text-fg',
  'after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:rounded-full after:bg-fg',
  'after:scale-x-0 after:transition-transform after:duration-(--dur-base) after:ease-out',
  'data-[status=active]:font-semibold data-[status=active]:text-fg data-[status=active]:after:scale-x-100',
  'focus-visible:outline-offset-[-2px]',
);

/** "People · Companies", the current one underlined. */
export function EntityTabs() {
  return (
    <nav
      aria-label={entitiesCopy.tabsLabel}
      className="mb-4 flex h-9 items-stretch gap-5 border-b border-border"
    >
      <Link to="/people" activeOptions={{ exact: true }} className={TAB_CLASS}>
        {entitiesCopy.people}
      </Link>
      <Link to="/companies" activeOptions={{ exact: true }} className={TAB_CLASS}>
        {entitiesCopy.companies}
      </Link>
    </nav>
  );
}
