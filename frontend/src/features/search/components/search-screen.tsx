/**
 * The results page (`/search`, ORG-08): the query, its `@` filters, and the
 * Meetings / Moments / People tabs with highlighted snippets. The URL holds
 * the whole state, so a search can be revisited from history.
 */
import type { ReactNode } from 'react';

import { Alert, AlertDescription } from '@/ui/alert';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Page } from '@/ui/page-header';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/ui/tabs';

import { searchCopy } from '../copy';
import { hitTarget } from '../hit-target';
import { useSearch } from '../queries';
import type { SearchPageParams, SearchTab } from '../search';
import type { FilterChip, Hit, SearchResults } from '../types';
import { FilterChips } from './filter-chips';
import { ResultList, ResultsSkeleton } from './result-list';
import { NoResults, SearchField, StartSearching } from './search-field';
import { useOpenHit } from './use-open-hit';

/** Hits of each kind the page asks for. */
const PAGE_LIMIT = 50;

/** Props of {@link SearchScreen}. */
export interface SearchScreenProps {
  params: SearchPageParams;
  /** Merges new parameters into the URL (replacing the history entry). */
  onParamsChange: (params: SearchPageParams) => void;
}

/** The tabs and which hits each one lists. */
const TABS: readonly { id: SearchTab; label: string; hits: (results: SearchResults) => Hit[] }[] = [
  { id: 'meetings', label: searchCopy.meetings, hits: (results) => results.meetings },
  { id: 'moments', label: searchCopy.moments, hits: (results) => results.moments },
  {
    id: 'people',
    label: searchCopy.people,
    hits: (results) => [...results.people, ...results.companies],
  },
];

/** The results page. */
export function SearchScreen({ params, onParamsChange }: SearchScreenProps) {
  const text = params.q ?? '';
  const filters = params.filters ?? [];
  return (
    <Page width="document">
      <SearchField
        value={text}
        onChange={(q) => {
          onParamsChange({ q });
        }}
      />
      <FilterChips
        filters={filters}
        onRemove={(removed) => {
          onParamsChange({ filters: filters.filter((chip) => chip !== removed) });
        }}
        className="pt-3"
      />
      <SearchBody params={params} onParamsChange={onParamsChange} />
    </Page>
  );
}

/** What the search found, by tab, or why there is nothing to show. */
function SearchBody({ params, onParamsChange }: SearchScreenProps) {
  const text = params.q ?? '';
  const filters = params.filters ?? [];
  const enabled = text.trim() !== '' || filters.length > 0;
  const request = { text, filters: { chips: filters, domains: [] }, limit: PAGE_LIMIT };
  const results = useSearch(request, enabled);
  const open = useResultOpener(filters, onParamsChange);
  if (!enabled) return <StartSearching />;
  if (results.isError) return <SearchError message={results.error.message} />;
  const toMoments = () => {
    onParamsChange({ tab: 'moments' });
  };
  return (
    <ResultTabs
      tab={params.tab ?? 'meetings'}
      results={results.data}
      onTabChange={(next) => {
        onParamsChange({ tab: next });
      }}
      onOpen={open}
      empty={(empty) => (
        <NoResults text={text.trim()} onMoments={empty === 'moments' ? null : toMoments} />
      )}
    />
  );
}

/** Opens a hit: people and companies become filters on the Meetings tab. */
function useResultOpener(
  filters: FilterChip[],
  onParamsChange: (params: SearchPageParams) => void,
) {
  const openHit = useOpenHit();
  return (hit: Hit) => {
    const target = hitTarget(hit);
    if (target.kind !== 'filter') {
      openHit(target);
      return;
    }
    const known = filters.some((chip) => chip.id === target.chip.id);
    onParamsChange({
      filters: known ? filters : [...filters, target.chip],
      q: '',
      tab: 'meetings',
    });
  };
}

interface ResultTabsProps {
  tab: SearchTab;
  results: SearchResults | undefined;
  onTabChange: (tab: SearchTab) => void;
  onOpen: (hit: Hit) => void;
  /** What a tab without hits shows. */
  empty: (tab: SearchTab) => ReactNode;
}

function ResultTabs({ tab, results, onTabChange, onOpen, empty }: ResultTabsProps) {
  const select = (value: string) => {
    const next = TABS.find((candidate) => candidate.id === value);
    if (next !== undefined) onTabChange(next.id);
  };
  return (
    <Tabs value={tab} onValueChange={select} className="pt-5">
      <TabsList aria-label={searchCopy.page.tabs}>
        {TABS.map(({ id, label, hits }) => (
          <TabsTrigger key={id} value={id}>
            {label}
            <span className="text-xs font-normal text-fg-subtle tabular-nums">
              {results === undefined ? '' : hits(results).length}
            </span>
          </TabsTrigger>
        ))}
      </TabsList>
      {TABS.map(({ id, hits }) => (
        <TabsContent key={id} value={id}>
          {results === undefined ? (
            <ResultsSkeleton />
          ) : hits(results).length === 0 ? (
            empty(id)
          ) : (
            <ResultList hits={hits(results)} moments={id === 'moments'} onOpen={onOpen} />
          )}
        </TabsContent>
      ))}
    </Tabs>
  );
}

function SearchError({ message }: { message: string }) {
  return (
    <Alert variant="destructive" className="mt-6">
      <CircleAlert {...ICON_PROPS} />
      <AlertDescription className="flex flex-col gap-1">
        <span>{searchCopy.page.error}</span>
        <span className="text-fg-muted">{message}</span>
      </AlertDescription>
    </Alert>
  );
}
