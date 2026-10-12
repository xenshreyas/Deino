import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Meetings from '@/lib/ipc/generated/meetings';
import { createQueryClient } from '@/lib/ipc';

import { RecentNotes } from './recent-notes';

const list = vi.hoisted(() => vi.fn<typeof Meetings.commands.list>());
vi.mock('@/lib/ipc/generated/meetings', () => ({ commands: { list } }));
vi.mock('@/features/onboarding', () => ({
  TryDeino: () => <ul aria-label="Try Deino" />,
}));

function meeting(id: string, title: string): Meetings.Meeting {
  return {
    id: id as Meetings.MeetingId,
    title,
    createdAt: '2026-10-08T14:00:00Z',
  } as Meetings.Meeting;
}

function renderNotes() {
  const root = createRootRoute();
  const home = createRoute({ getParentRoute: () => root, path: '/', component: RecentNotes });
  const others = ['/meetings', '/meetings/$meetingId'].map((path) =>
    createRoute({ getParentRoute: () => root, path }),
  );
  const router = createRouter({
    routeTree: root.addChildren([home, ...others]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

describe('Home’s Recent notes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('offers a test recording and the sample meeting before the first note (UX-24)', async () => {
    list.mockResolvedValue({ status: 'ok', data: [] });
    renderNotes();

    expect(await screen.findByRole('list', { name: 'Try Deino' })).toBeTruthy();
  });

  it('lists the notes, and offers nothing else, once there are some', async () => {
    list.mockResolvedValue({ status: 'ok', data: [meeting('m-1', 'Pricing call')] });
    renderNotes();

    expect(await screen.findByRole('link', { name: /Pricing call/ })).toBeTruthy();
    expect(screen.queryByRole('list', { name: 'Try Deino' })).toBeNull();
  });
});
