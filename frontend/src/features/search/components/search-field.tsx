/**
 * The results page's search field and its resting and empty states.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { EmptyState } from '@/ui/empty-state';
import { fieldClassName } from '@/ui/input';
import { ICON_PROPS, Search } from '@/ui/icons';

import { searchCopy } from '../copy';

/** Props of {@link SearchField}. */
export interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
}

/** A 44px field with a search icon; results follow as the user types. */
export function SearchField({ value, onChange }: SearchFieldProps) {
  return (
    <label className="relative block">
      <span className="sr-only">{searchCopy.page.title}</span>
      <Search
        {...ICON_PROPS}
        size={18}
        className="pointer-events-none absolute top-1/2 left-3.5 -translate-y-1/2 text-fg-muted"
      />
      <input
        type="search"
        autoFocus
        value={value}
        placeholder={searchCopy.page.placeholder}
        onChange={(event) => {
          onChange(event.target.value);
        }}
        className={cn(
          fieldClassName,
          'h-11 pr-3 pl-10 text-base [&::-webkit-search-cancel-button]:appearance-none',
        )}
      />
    </label>
  );
}

/** Before anything is typed: what search can do. */
export function StartSearching() {
  return (
    <EmptyState
      icon={Search}
      title={searchCopy.page.start}
      description={searchCopy.page.startBody}
      className="pt-[14vh]"
    />
  );
}

/** Props of {@link NoResults}. */
export interface NoResultsProps {
  text: string;
  /** Switches to the Moments tab, or `null` when already there. */
  onMoments: (() => void) | null;
}

/** A tab with no hits: say so, and offer the transcripts. */
export function NoResults({ text, onMoments }: NoResultsProps) {
  return (
    <EmptyState
      icon={Search}
      title={searchCopy.page.empty(text)}
      description={searchCopy.page.emptyBody}
      className="pt-[10vh]"
      action={
        onMoments === null ? undefined : (
          <Button variant="secondary" onClick={onMoments}>
            {searchCopy.page.emptyMoments}
          </Button>
        )
      }
    />
  );
}
