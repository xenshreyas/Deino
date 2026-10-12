import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { currentSettings as defaultSettings } from '@/dev/mock-ipc/settings-store';
import type * as ActionItems from '@/lib/ipc/generated/action-items';
import type * as Generated from '@/lib/ipc/generated/meetings';
import type * as SettingsBindings from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { usePageStore } from '../page-store';
import { ALL_MEETINGS, type ListedMeeting, type MeetingId } from '../types';
import { MeetingsListScreen } from './meetings-list-screen';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      get: vi.fn<typeof Generated.commands.get>(),
      rename: vi.fn<typeof Generated.commands.rename>(),
      page: vi.fn<typeof Generated.commands.page>(),
      matchingIds: vi.fn<typeof Generated.commands.matchingIds>(),
      views: vi.fn<typeof Generated.commands.views>(),
      createView: vi.fn<typeof Generated.commands.createView>(),
      updateView: vi.fn<typeof Generated.commands.updateView>(),
      deleteView: vi.fn<typeof Generated.commands.deleteView>(),
      trash: vi.fn<typeof Generated.commands.trash>(),
      restore: vi.fn<typeof Generated.commands.restore>(),
      trashed: vi.fn<typeof Generated.commands.trashed>(),
      deleteForever: vi.fn<typeof Generated.commands.deleteForever>(),
      tagMany: vi.fn<typeof Generated.commands.tagMany>(),
      untagMany: vi.fn<typeof Generated.commands.untagMany>(),
      fileMany: vi.fn<typeof Generated.commands.fileMany>(),
      merge: vi.fn<typeof Generated.commands.merge>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/meetings', () => ({ commands }));
const itemCommands = vi.hoisted(() => ({
  filterOptions: vi.fn<typeof ActionItems.commands.filterOptions>(),
}));
vi.mock('@/lib/ipc/generated/action-items', () => ({ commands: itemCommands, events: {} }));
const settingsCommands = vi.hoisted(() => ({
  get: vi.fn<typeof SettingsBindings.commands.get>(),
  patch: vi.fn<typeof SettingsBindings.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settingsCommands, events: {} }));

function row(id: string, title: string): ListedMeeting {
  return {
    id: id as MeetingId,
    title,
    createdAt: new Date().toISOString(),
    startedAt: null,
    duration: 1_800,
    status: 'ready',
    attendees: [
      {
        personId: 'p-maya' as Generated.PersonId,
        name: 'Maya Chen',
        avatarPath: null,
        isMe: false,
      },
    ],
    tags: [{ id: 't-1' as Generated.TagId, name: 'Customer', color: 2 }],
  };
}

function renderScreen() {
  const root = createRootRoute();
  const page = createRoute({
    getParentRoute: () => root,
    path: '/meetings',
    component: () => (
      <MeetingsListScreen
        tags={[]}
        onStartRecording={vi.fn()}
        shortcuts={{ newNote: '⌘N', record: '⌘R' }}
      />
    ),
  });
  const others = ['/meetings/$meetingId', '/meetings/trash', '/'].map((path) =>
    createRoute({ getParentRoute: () => root, path }),
  );
  const router = createRouter({
    routeTree: root.addChildren([page, ...others]),
    history: createMemoryHistory({ initialEntries: ['/meetings'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('Meetings page', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    usePageStore.getState().reset(ALL_MEETINGS);
    settingsCommands.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
    settingsCommands.patch.mockResolvedValue({ status: 'ok', data: defaultSettings() });
    itemCommands.filterOptions.mockResolvedValue({
      status: 'ok',
      data: { people: [], companies: [], tags: [], folders: [] },
    });
    commands.views.mockResolvedValue({ status: 'ok', data: [] });
    commands.page.mockResolvedValue({
      status: 'ok',
      data: { meetings: [row('m-2', 'Pricing call'), row('m-1', 'Standup')], next: null },
    });
    commands.matchingIds.mockResolvedValue({
      status: 'ok',
      data: ['m-2', 'm-1', 'm-0'] as MeetingId[],
    });
    commands.trash.mockResolvedValue({ status: 'ok', data: { changed: ['m-2'] as MeetingId[] } });
  });

  test('rows sit under today with their tag, avatar and length', async () => {
    renderScreen();

    const today = await screen.findByRole('region', { name: 'Today' });
    expect(within(today).getByText('Pricing call')).toBeDefined();
    expect(within(today).getAllByText('Customer')).toHaveLength(2);
    expect(within(today).getAllByText('30m')).toHaveLength(2);
    expect(commands.page).toHaveBeenCalledWith(ALL_MEETINGS, null, 50);
  });

  test('a ticked row opens the bulk bar, whose Delete moves it to the trash', async () => {
    renderScreen();

    fireEvent.click(await screen.findByRole('checkbox', { name: 'Select Pricing call' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected meetings' });
    expect(within(bar).getByText('1 meeting selected')).toBeDefined();
    fireEvent.click(within(bar).getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(commands.trash).toHaveBeenCalledWith(['m-2']);
    });
  });

  test('⌘A selects every matching meeting, including those not loaded', async () => {
    renderScreen();
    await screen.findByText('Pricing call');

    fireEvent.keyDown(window, { key: 'a', metaKey: true });

    expect(await screen.findByText('3 meetings selected')).toBeDefined();
    expect(screen.getByRole('button', { name: /Run automation/ }).hasAttribute('disabled')).toBe(
      true,
    );
  });
});
