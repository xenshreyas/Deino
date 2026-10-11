import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/search';
import { createQueryClient } from '@/lib/ipc';
import { Command, CommandInput, CommandList } from '@/ui/command';

import type { FilterChip, Hit, SearchResults } from '../types';
import { FilterChips } from './filter-chips';
import { FilterPalette } from './palette-filters';
import { SearchPalette } from './palette-results';

const commands = vi.hoisted(() => ({
  search: vi.fn<typeof Generated.commands.search>(),
  suggestFilters: vi.fn<typeof Generated.commands.suggestFilters>(),
}));
vi.mock('@/lib/ipc/generated/search', () => ({ commands, events: {} }));

const MAYA: FilterChip = { kind: 'person', id: 'person-maya', label: 'Maya Chen' };

function moment(): Hit {
  return {
    kind: 'segment',
    refId: 'segment-1',
    meetingId: asMeetingId('meeting-a'),
    tsMs: 61_000,
    endMs: null,
    label: { text: 'Q4 roadmap review', highlights: [] },
    snippet: { text: 'ship the beta in November', highlights: [{ start: 9, end: 13 }] },
    date: '2026-10-08T09:00:00Z',
    score: 3,
  };
}

function results(overrides: Partial<SearchResults>): SearchResults {
  const empty = {
    meetings: [],
    moments: [],
    people: [],
    companies: [],
    actionItems: [],
    folders: [],
  };
  return { ...empty, filters: { chips: [], domains: [] }, semantic: false, ...overrides };
}

/** The palette's search state around the two providers. */
function Palette({ close }: { close: () => void }) {
  const [search, setSearch] = useState('');
  const [filters, setFilters] = useState<FilterChip[]>([]);
  const props = { search, close, filters, onFiltersChange: setFilters, onSearchChange: setSearch };
  const remove = (chip: FilterChip) => {
    setFilters(filters.filter((filter) => filter !== chip));
  };
  return (
    <Command>
      <CommandInput
        value={search}
        onValueChange={setSearch}
        aria-label="Search"
        before={<FilterChips filters={filters} onRemove={remove} />}
      />
      <CommandList>
        <FilterPalette {...props} />
        <SearchPalette {...props} />
      </CommandList>
    </Command>
  );
}

function renderPalette() {
  const close = vi.fn();
  const root = createRootRoute({
    component: () => (
      <>
        <Palette close={close} />
        <Outlet />
      </>
    ),
  });
  const page = (path: string) =>
    createRoute({ getParentRoute: () => root, path, component: () => null });
  const router = createRouter({
    routeTree: root.addChildren([page('/'), page('/meetings/$meetingId'), page('/search')]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return { router, close };
}

describe('the search palette', () => {
  beforeEach(() => {
    commands.search.mockResolvedValue({ status: 'ok', data: results({ moments: [moment()] }) });
    commands.suggestFilters.mockResolvedValue({ status: 'ok', data: [MAYA] });
  });

  test('a moment shows its meeting, time and highlighted words, and opens the meeting there', async () => {
    const { router, close } = renderPalette();

    fireEvent.change(await screen.findByLabelText('Search'), { target: { value: 'beta' } });

    const option = await screen.findByRole('option', { name: /Q4 roadmap review/ });
    expect(option.textContent).toContain('1:01');
    expect(option.querySelector('mark')?.textContent).toBe('beta');
    fireEvent.click(option);
    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/meetings/meeting-a');
    });
    expect(router.state.location.search).toEqual({ tab: 'transcript', t: 61_000 });
    expect(close).toHaveBeenCalled();
  });

  test('@ suggests filters; choosing one becomes a chip and scopes the search', async () => {
    renderPalette();
    const input = await screen.findByLabelText('Search');

    fireEvent.change(input, { target: { value: 'roadmap @ma' } });
    fireEvent.click(await screen.findByRole('option', { name: /Maya Chen/ }));

    expect(await screen.findByRole('button', { name: 'Remove filter Maya Chen' })).toBeDefined();
    expect((input as HTMLInputElement).value).toBe('roadmap ');
    await waitFor(() => {
      expect(commands.search).toHaveBeenLastCalledWith({
        text: 'roadmap ',
        filters: { chips: [MAYA], domains: [] },
        limit: 5,
      });
    });
    expect(commands.suggestFilters).toHaveBeenCalledWith('ma', null);
  });

  test('"View all results" opens the results page with the text', async () => {
    const { router } = renderPalette();

    fireEvent.change(await screen.findByLabelText('Search'), { target: { value: 'beta' } });
    fireEvent.click(await screen.findByRole('option', { name: /View all results for “beta”/ }));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/search');
    });
    expect(router.state.location.search).toEqual({ q: 'beta', filters: [] });
  });
});
