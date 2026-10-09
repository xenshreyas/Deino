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

import type * as Generated from '@/lib/ipc/generated/meetings';
import { createQueryClient } from '@/lib/ipc';

import { asMeetingId, type Meeting } from '../types';
import { MeetingsScreen } from './meetings-screen';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      get: vi.fn<typeof Generated.commands.get>(),
      rename: vi.fn<typeof Generated.commands.rename>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/meetings', () => ({ commands }));
const deleteMeeting = vi.hoisted(() => vi.fn<(id: string) => Promise<void>>());
vi.mock('@/services/storageService', () => ({ storageService: { deleteMeeting } }));

const standup: Meeting = {
  id: asMeetingId('meeting-1'),
  title: 'Weekly standup',
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

/** The screen at `/meetings` inside a router and a query client. */
function renderScreen(onStartRecording = vi.fn()) {
  const root = createRootRoute();
  const list = createRoute({
    getParentRoute: () => root,
    path: '/meetings',
    component: () => (
      <MeetingsScreen
        onStartRecording={onStartRecording}
        shortcuts={{ newNote: '⌘N', record: '⌘⇧R' }}
      />
    ),
  });
  const pages = ['/', '/meetings/$meetingId'].map((path) =>
    createRoute({ getParentRoute: () => root, path, component: () => null }),
  );
  const router = createRouter({
    routeTree: root.addChildren([list, ...pages]),
    history: createMemoryHistory({ initialEntries: ['/meetings'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return { onStartRecording };
}

describe('Meetings page', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  test('with no meetings it offers to start recording', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [] });
    const { onStartRecording } = renderScreen();

    fireEvent.click(await screen.findByRole('button', { name: /Start recording/ }));
    expect(screen.getByText('No meetings yet')).toBeDefined();
    expect(onStartRecording).toHaveBeenCalledTimes(1);
  });

  test('lists meetings under their date header, each linking to its page', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [standup] });
    renderScreen();

    const link = await screen.findByRole('link', { name: /Weekly standup/ });
    expect(link.getAttribute('href')).toBe('/meetings/meeting-1');
    expect(screen.getByRole('heading', { name: 'Today' })).toBeDefined();
  });

  test('deleting asks first, then removes the meeting from the list', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [standup] });
    deleteMeeting.mockResolvedValue();
    renderScreen();

    fireEvent.pointerDown(
      await screen.findByRole('button', { name: 'Actions for Weekly standup' }),
      {
        button: 0,
        ctrlKey: false,
      },
    );
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Delete' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(deleteMeeting).toHaveBeenCalledWith('meeting-1');
    });
    await waitFor(() => {
      expect(screen.queryByRole('link', { name: /Weekly standup/ })).toBeNull();
    });
  });

  test('a load failure is shown inline with a retry', async () => {
    commands.list.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'could not load or save meetings' },
    });
    renderScreen();

    expect(await screen.findByText('Could not load your meetings.')).toBeDefined();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeDefined();
  });
});
