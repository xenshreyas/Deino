/**
 * The `@` filters as removable chips: in the palette's input row and above
 * the results page's tabs.
 */
import { cn } from '@/ui/cn';
import {
  Building2,
  Folder,
  type Icon,
  ICON_PROPS,
  NotebookText,
  Tag,
  UserRound,
  X,
} from '@/ui/icons';

import { searchCopy } from '../copy';
import type { FilterChip, FilterKind } from '../types';

/** The icon of each filter kind. */
export const FILTER_ICONS: Record<FilterKind, Icon> = {
  person: UserRound,
  company: Building2,
  tag: Tag,
  folder: Folder,
  meeting: NotebookText,
};

/** Props of {@link FilterChips}. */
export interface FilterChipsProps {
  filters: readonly FilterChip[];
  onRemove: (chip: FilterChip) => void;
  className?: string;
}

/** Each filter as a 24px chip with a remove button. */
export function FilterChips({ filters, onRemove, className }: FilterChipsProps) {
  if (filters.length === 0) return null;
  return (
    <ul className={cn('flex flex-wrap items-center gap-1.5', className)}>
      {filters.map((chip) => {
        const ChipIcon = FILTER_ICONS[chip.kind];
        return (
          <li
            key={`${chip.kind}:${chip.id}`}
            className="inline-flex h-6 max-w-48 items-center gap-1 rounded-full bg-bg-muted pr-0.5 pl-2 text-xs text-fg"
          >
            <ChipIcon {...ICON_PROPS} size={12} className="shrink-0 text-fg-muted" />
            <span className="truncate">{chip.label}</span>
            <button
              type="button"
              aria-label={searchCopy.removeFilter(chip.label)}
              className="grid size-5 shrink-0 place-items-center rounded-full text-fg-muted transition-colors duration-(--dur-fast) ease-out hover:bg-border hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
              onClick={() => {
                onRemove(chip);
              }}
            >
              <X {...ICON_PROPS} size={12} />
            </button>
          </li>
        );
      })}
    </ul>
  );
}
