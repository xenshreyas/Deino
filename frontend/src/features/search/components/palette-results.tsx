/**
 * ⌘K search results (ORG-08): meetings, moments, people, companies and
 * action items for what the user typed and the `@` filters they picked.
 * Registered in the shell's `command-palette/providers.ts`.
 */
import type { ReactNode } from 'react';

import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import {
  AudioLines,
  Building2,
  type Icon,
  ICON_PROPS,
  ListChecks,
  NotebookText,
  Search,
  UserRound,
} from '@/ui/icons';

import { searchCopy } from '../copy';
import { activeMention, momentTime, shortDate } from '../format';
import { hitKey, hitTarget } from '../hit-target';
import { useSearch } from '../queries';
import type { FilterChip, Hit, SearchResults } from '../types';
import { HighlightedText } from './highlighted-text';
import { useOpenHit } from './use-open-hit';

/** Hits of each kind the palette shows. */
const PALETTE_LIMIT = 5;
/** Characters before a search runs without filters. */
const MIN_QUERY = 2;

/** What the palette gives its search providers (structurally the shell's props). */
export interface SearchPaletteProps {
  search: string;
  close: () => void;
  filters: readonly FilterChip[];
  onFiltersChange: (filters: FilterChip[]) => void;
  onSearchChange: (search: string) => void;
}

/** The section groups, in order. */
const SECTIONS: readonly {
  key: keyof Pick<SearchResults, 'meetings' | 'moments' | 'people' | 'companies' | 'actionItems'>;
  heading: string;
  icon: Icon;
}[] = [
  { key: 'meetings', heading: searchCopy.meetings, icon: NotebookText },
  { key: 'moments', heading: searchCopy.moments, icon: AudioLines },
  { key: 'people', heading: searchCopy.people, icon: UserRound },
  { key: 'companies', heading: searchCopy.companies, icon: Building2 },
  { key: 'actionItems', heading: searchCopy.actionItems, icon: ListChecks },
];

/** The search's results as palette groups, plus "View all results". */
export function SearchPalette(props: SearchPaletteProps) {
  const { search, filters } = props;
  const text = (activeMention(search)?.rest ?? search).trimStart();
  const enabled = text.trim().length >= MIN_QUERY || filters.length > 0;
  const request = { text, filters: { chips: [...filters], domains: [] }, limit: PALETTE_LIMIT };
  const { data } = useSearch(request, enabled);
  if (!enabled || data === undefined) return null;
  return (
    <>
      {SECTIONS.map(({ key, heading, icon }) => (
        <HitGroup
          key={key}
          section={key}
          heading={heading}
          icon={icon}
          hits={data[key]}
          palette={props}
        />
      ))}
      <ViewAllItem text={text.trim()} palette={props} />
    </>
  );
}

function HitGroup({
  section,
  heading,
  icon,
  hits,
  palette,
}: {
  section: string;
  heading: string;
  icon: Icon;
  hits: readonly Hit[];
  palette: SearchPaletteProps;
}) {
  const open = usePaletteOpen(palette);
  if (hits.length === 0) return null;
  return (
    <CommandGroup heading={heading}>
      {hits.map((hit) => (
        <CommandItem
          key={hitKey(hit)}
          value={`search ${section} ${hitKey(hit)}`}
          keywords={[palette.search]}
          onSelect={() => {
            open(hit);
          }}
        >
          <HitRow hit={hit} icon={icon} moment={section === 'moments'} />
        </CommandItem>
      ))}
    </CommandGroup>
  );
}

/** Runs a hit: people and companies become filters, the rest navigate. */
function usePaletteOpen(palette: SearchPaletteProps) {
  const openHit = useOpenHit();
  return (hit: Hit) => {
    const target = hitTarget(hit);
    if (target.kind === 'filter') {
      const known = palette.filters.some((chip) => chip.id === target.chip.id);
      palette.onFiltersChange(known ? [...palette.filters] : [...palette.filters, target.chip]);
      palette.onSearchChange('');
      return;
    }
    palette.close();
    openHit(target);
  };
}

/** Icon, label and meta on the first line; the matched snippet below. */
function HitRow({ hit, icon: HitIcon, moment }: { hit: Hit; icon: Icon; moment: boolean }) {
  return (
    <>
      <HitIcon {...ICON_PROPS} />
      <span className="flex min-w-0 flex-1 flex-col py-1.5">
        <HighlightedText snippet={hit.label} className="truncate" />
        {hit.snippet.text === '' ? null : (
          <HighlightedText snippet={hit.snippet} className="truncate text-xs text-fg-muted" />
        )}
      </span>
      <CommandShortcut className="tabular-nums">{hitMeta(hit, moment)}</CommandShortcut>
    </>
  );
}

/** A moment's time in the recording, else its meeting's date. */
function hitMeta(hit: Hit, moment: boolean): ReactNode {
  if (moment && hit.tsMs !== null) return momentTime(hit.tsMs);
  return shortDate(hit.date);
}

function ViewAllItem({ text, palette }: { text: string; palette: SearchPaletteProps }) {
  const openHit = useOpenHit();
  return (
    <CommandGroup>
      <CommandItem
        value="search view-all"
        keywords={[palette.search]}
        onSelect={() => {
          palette.close();
          openHit({ kind: 'results', text, filters: [...palette.filters] });
        }}
      >
        <Search {...ICON_PROPS} />
        <span className="truncate">{searchCopy.viewAll(text)}</span>
        <CommandShortcut>↵</CommandShortcut>
      </CommandItem>
    </CommandGroup>
  );
}
