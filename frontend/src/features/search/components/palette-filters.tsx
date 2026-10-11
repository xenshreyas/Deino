/**
 * ⌘K `@` filters (Circleback §9.1): typing `@` lists people, companies,
 * tags, folders and meetings; choosing one turns it into a filter chip and
 * removes the `@word` from the search.
 */
import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import { ICON_PROPS } from '@/ui/icons';

import { searchCopy } from '../copy';
import { activeMention } from '../format';
import { useFilterSuggestions } from '../queries';
import type { FilterChip } from '../types';
import { FILTER_ICONS } from './filter-chips';
import type { SearchPaletteProps } from './palette-results';

/** The `@` suggestions while the user types a mention. */
export function FilterPalette({
  search,
  filters,
  onFiltersChange,
  onSearchChange,
}: SearchPaletteProps) {
  const mention = activeMention(search);
  const { data: suggestions = [] } = useFilterSuggestions(mention?.prefix ?? '', mention !== null);
  if (mention === null) return null;
  const choose = (chip: FilterChip) => {
    if (!filters.some((filter) => filter.kind === chip.kind && filter.id === chip.id)) {
      onFiltersChange([...filters, chip]);
    }
    onSearchChange(mention.rest);
  };
  return (
    <CommandGroup heading={searchCopy.filters}>
      {suggestions.map((chip) => {
        const ChipIcon = FILTER_ICONS[chip.kind];
        return (
          <CommandItem
            key={`${chip.kind}:${chip.id}`}
            value={`filter ${chip.kind} ${chip.id}`}
            keywords={[search]}
            onSelect={() => {
              choose(chip);
            }}
          >
            <ChipIcon {...ICON_PROPS} />
            <span className="truncate">{chip.label}</span>
            <CommandShortcut>{searchCopy.filterKinds[chip.kind]}</CommandShortcut>
          </CommandItem>
        );
      })}
    </CommandGroup>
  );
}
