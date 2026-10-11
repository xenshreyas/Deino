import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/search';
import { createQueryClient } from '@/lib/ipc';

import type { SearchPageParams } from '../search';
import type { Hit, SearchResults } from '../types';
import { SearchScreen } from './search-screen';

const commands = vi.hoisted(() => ({ search: vi.fn<typeof Generated.commands.search>() }));
vi.mock('@/lib/ipc/generated/search', () => ({ commands, events: {} }));

function hit(kind: Hit['kind'], text: string): Hit {
  return {
    kind,
    refId: `${kind}-1`,
    meetingId: kind === 'person' ? null : asMeetingId('meeting-a'),
    tsMs: kind === 'segment' ? 3_723_000 : null,
    endMs: null,
    label: { text: kind === 'person' ? 'Maya Chen' : 'Q4 planning', highlights: [] },
    snippet: { text, highlights: [{ start: 0, end: 7 }] },
    date: '2026-10-08T09:00:00Z',
    score: 1,
  };
}

function results(): SearchResults {
  return {
    meetings: [hit('note', 'roadmap slipped a week')],
    moments: [hit('segment', 'roadmap review on Friday')],
    people: [hit('person', '')],
    companies: [],
    actionItems: [],
    folders: [],
    filters: { chips: [], domains: [] },
    semantic: false,
  };
}

function renderScreen(params: SearchPageParams) {
  const onParamsChange = vi.fn();
  const root = createRootRoute();
  const page = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: () => <SearchScreen params={params} onParamsChange={onParamsChange} />,
  });
  const meeting = createRoute({ getParentRoute: () => root, path: '/meetings/$meetingId' });
  const person = createRoute({ getParentRoute: () => root, path: '/people/$personId' });
  const router = createRouter({
    routeTree: root.addChildren([page, meeting, person]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return { onParamsChange, router };
}

describe('the results page', () => {
  beforeEach(() => {
    commands.search.mockResolvedValue({ status: 'ok', data: results() });
  });

  test('tabs count their hits and the meetings tab highlights the match', async () => {
    renderScreen({ q: 'roadmap' });

    expect(await screen.findByRole('tab', { name: /Meetings\s*1/ })).toBeDefined();
    expect(screen.getByRole('tab', { name: /Moments\s*1/ })).toBeDefined();
    expect(screen.getByRole('tab', { name: /People\s*1/ })).toBeDefined();
    const mark = await screen.findByText('roadmap', { selector: 'mark' });
    expect(mark.parentElement?.textContent).toBe('roadmap slipped a week');
    expect(commands.search).toHaveBeenCalledWith({
      text: 'roadmap',
      filters: { chips: [], domains: [] },
      limit: 50,
    });
  });

  test('a moment shows its time and opens its meeting there', async () => {
    const { router } = renderScreen({ q: 'roadmap', tab: 'moments' });

    fireEvent.click(await screen.findByRole('button', { name: /1:02:03/ }));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/meetings/meeting-a');
    });
    expect(router.state.location.search).toEqual({ tab: 'transcript', t: 3_723_000 });
  });

  test('choosing a person opens their page', async () => {
    const { router } = renderScreen({ q: 'maya', tab: 'people' });

    fireEvent.click(await screen.findByRole('button', { name: /Maya Chen/ }));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/people/person-1');
    });
  });

  test('nothing found says so and offers the transcripts', async () => {
    commands.search.mockResolvedValue({
      status: 'ok',
      data: { ...results(), meetings: [], moments: [], people: [] },
    });
    const { onParamsChange } = renderScreen({ q: 'zebra' });

    expect(await screen.findByText('No results for “zebra”')).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: 'Search transcripts instead' }));
    expect(onParamsChange).toHaveBeenCalledWith({ tab: 'moments' });
  });

  test('an empty search explains the query language without searching', async () => {
    renderScreen({});

    expect(await screen.findByText('Search everything you have recorded')).toBeDefined();
    expect(commands.search).not.toHaveBeenCalled();
  });
});
