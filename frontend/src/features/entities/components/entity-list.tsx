/**
 * The People and Companies lists' shared parts (ui.md §3 List row): the
 * search field and sort switch above the list, the two-line rows, their
 * skeleton, and the "nobody matches" state.
 */
import type { ReactNode } from 'react';

import { Button } from '@/ui/button';
import { ICON_PROPS, Search, X } from '@/ui/icons';
import { SegmentedControl } from '@/ui/segmented-control';
import { Skeleton } from '@/ui/skeleton';

import { entitiesCopy } from '../copy';
import type { EntitySort } from '../types';

/** Props of {@link ListToolbar}. */
export interface ListToolbarProps {
  query: string;
  onQueryChange: (query: string) => void;
  sort: EntitySort;
  onSortChange: (sort: EntitySort) => void;
  placeholder: string;
}

const SORTS = [
  { value: 'recent', label: entitiesCopy.sortRecent },
  { value: 'name', label: entitiesCopy.sortName },
] as const;

/** Search on the left, sort on the right; Escape clears the search. */
export function ListToolbar({
  query,
  onQueryChange,
  sort,
  onSortChange,
  placeholder,
}: ListToolbarProps) {
  return (
    <div className="flex items-center gap-3 pb-3">
      <label className="flex h-8 min-w-0 flex-1 items-center gap-2 rounded-md bg-bg-muted px-2.5 text-fg-muted focus-within:ring-2 focus-within:ring-focus-ring">
        <Search {...ICON_PROPS} className="shrink-0" />
        <input
          type="search"
          value={query}
          placeholder={placeholder}
          aria-label={placeholder}
          onChange={(event) => {
            onQueryChange(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === 'Escape') onQueryChange('');
          }}
          className="h-full min-w-0 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-subtle [&::-webkit-search-cancel-button]:hidden"
        />
      </label>
      <SegmentedControl
        label={entitiesCopy.sortLabel}
        segments={SORTS}
        value={sort}
        onValueChange={onSortChange}
      />
    </div>
  );
}

/** Row-shaped placeholders while a list loads: a disc and two lines. */
export function ListSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1">
      {Array.from({ length: 6 }, (_, index) => (
        <div key={index} className="flex h-14 items-center gap-3 px-3">
          <Skeleton className="size-6 rounded-full" />
          <div className="flex flex-1 flex-col gap-2">
            <Skeleton className="h-2.5 w-1/3" />
            <Skeleton className="h-2 w-1/5" />
          </div>
          <Skeleton className="h-2.5 w-16" />
        </div>
      ))}
    </div>
  );
}

/** The search found nothing: say so and offer to clear it. */
export function NoMatches({ message, onClear }: { message: string; onClear: () => void }) {
  return (
    <div className="flex flex-col items-center gap-3 py-16 text-center">
      <p className="text-sm text-fg-muted">{message}</p>
      <Button variant="secondary" size="sm" onClick={onClear}>
        <X {...ICON_PROPS} />
        {entitiesCopy.clearSearch}
      </Button>
    </div>
  );
}

/** Props of {@link EntityRowBody}. */
export interface EntityRowBodyProps {
  avatar: ReactNode;
  name: string;
  /** The second line: title · company, or domain · people. */
  detail: string;
  /** When they last met, already worded. */
  when: string;
  /** How many meetings, already worded. */
  count: string;
}

/** The inside of a two-line row (56px): avatar, name and detail, then when and how often. */
export function EntityRowBody({ avatar, name, detail, when, count }: EntityRowBodyProps) {
  return (
    <>
      {avatar}
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-sm text-fg">{name}</span>
        {detail === '' ? null : <span className="truncate text-xs text-fg-muted">{detail}</span>}
      </span>
      <span className="flex shrink-0 flex-col items-end text-xs tabular-nums">
        <span className="text-fg-muted">{when}</span>
        <span className="text-fg-subtle">{count}</span>
      </span>
    </>
  );
}

/** Classes of a list row's link. */
export const ROW_CLASS =
  'flex h-14 items-center gap-3 rounded-md px-3 transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:bg-bg-muted focus-visible:outline-offset-[-2px]';
