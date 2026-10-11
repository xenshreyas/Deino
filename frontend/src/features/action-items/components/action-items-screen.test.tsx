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

import type * as Generated from '@/lib/ipc/generated/action-items';
import type * as SettingsBindings from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { usePageStore } from '../page-store';
import type { ActionItemId, ActionItemView, FilterOptions, PersonId } from '../types';
import { ActionItemsScreen } from './action-items-screen';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      today: vi.fn<typeof Generated.commands.today>(),
      create: vi.fn<typeof Generated.commands.create>(),
      update: vi.fn<typeof Generated.commands.update>(),
      bulkUpdate: vi.fn<typeof Generated.commands.bulkUpdate>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      markAllMineDone: vi.fn<typeof Generated.commands.markAllMineDone>(),
      filterOptions: vi.fn<typeof Generated.commands.filterOptions>(),
      views: vi.fn<typeof Generated.commands.views>(),
      createView: vi.fn<typeof Generated.commands.createView>(),
      updateView: vi.fn<typeof Generated.commands.updateView>(),
      deleteView: vi.fn<typeof Generated.commands.deleteView>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/action-items', () => ({ commands, events: {} }));
const settingsCommands = vi.hoisted(() => ({
  get: vi.fn<typeof SettingsBindings.commands.get>(),
  patch: vi.fn<typeof SettingsBindings.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settingsCommands, events: {} }));

const ME = 'person-me' as PersonId;
const MAYA = 'person-maya' as PersonId;
const OPTIONS: FilterOptions = {
  people: [
    { id: ME, name: 'Shreyas Rao', email: 'shreyas@deino.dev', isMe: true },
    { id: MAYA, name: 'Maya Chen', email: 'maya@deino.dev', isMe: false },
  ],
  companies: [],
  tags: [],
  folders: [],
};

/** Noon, `days` from today, local: how Rust stores due days. */
function dueIn(days: number): string {
  const day = new Date();
  day.setDate(day.getDate() + days);
  day.setHours(12, 0, 0, 0);
  return day.toISOString();
}

function item(id: string, title: string, days: number): ActionItemView {
  const now = new Date().toISOString();
  return {
    id: id as ActionItemId,
    meetingId: null,
    title,
    description: '',
    assigneePersonId: ME,
    dueAt: dueIn(days),
    status: 'open',
    priority: null,
    sourceSegmentId: null,
    completedAt: null,
    createdBy: 'ai',
    createdAt: now,
    updatedAt: now,
    meetingTitle: null,
    meetingHeldAt: null,
    assigneeName: 'Shreyas Rao',
    assigneeIsMe: true,
    sourceStart: null,
  };
}

const late = item('item-late', 'Send the deck', -2);
const soon = item('item-soon', 'Book the room', 3);

function renderScreen() {
  const root = createRootRoute();
  const page = createRoute({
    getParentRoute: () => root,
    path: '/action-items',
    component: () => <ActionItemsScreen choice={{ kind: 'mine' }} onChoose={vi.fn()} />,
  });
  const meeting = createRoute({ getParentRoute: () => root, path: '/meetings/$meetingId' });
  const router = createRouter({
    routeTree: root.addChildren([page, meeting]),
    history: createMemoryHistory({ initialEntries: ['/action-items'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('Action Items page', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    usePageStore.getState().reset();
    commands.list.mockResolvedValue({ status: 'ok', data: [late, soon] });
    commands.today.mockResolvedValue({ status: 'ok', data: { overdue: [late], dueToday: [] } });
    commands.filterOptions.mockResolvedValue({ status: 'ok', data: OPTIONS });
    commands.views.mockResolvedValue({ status: 'ok', data: [] });
    commands.update.mockResolvedValue({ status: 'ok', data: { ...late, status: 'done' } });
    commands.bulkUpdate.mockResolvedValue({ status: 'ok', data: 2 });
    // The page menu's reminder switch waits for the settings; these tests never open it.
    settingsCommands.get.mockReturnValue(new Promise(() => undefined));
  });

  test('my open items are grouped by due date under the Today digest', async () => {
    renderScreen();

    const overdue = await screen.findByRole('region', { name: 'Overdue' });
    expect(within(overdue).getByText('Send the deck')).toBeDefined();
    expect(screen.getByRole('button', { name: '1 overdue' })).toBeDefined();
    expect(commands.list).toHaveBeenCalledWith(
      expect.objectContaining({ assignee: { kind: 'me' }, status: 'open' }),
    );
  });

  test('ticking an item draws the check, then completes it', async () => {
    renderScreen();

    const box = await screen.findByRole('checkbox', { name: 'Mark “Send the deck” done' });
    fireEvent.click(box);

    expect(box.getAttribute('aria-checked')).toBe('true');
    await waitFor(() => {
      expect(commands.update).toHaveBeenCalledWith(late.id, { field: 'status', status: 'done' });
    });
  });

  test('F opens the filter menu, and Overdue narrows the list', async () => {
    renderScreen();
    await screen.findByText('Send the deck');

    fireEvent.keyDown(window, { key: 'f' });
    fireEvent.click(await screen.findByRole('option', { name: 'Overdue' }));

    await waitFor(() => {
      expect(commands.list).toHaveBeenLastCalledWith(expect.objectContaining({ due: 'overdue' }));
    });
    expect(screen.getByRole('button', { name: 'Remove filter: Overdue' })).toBeDefined();
  });

  test('click and shift-click select a range, and the bar reassigns it', async () => {
    renderScreen();

    fireEvent.click(await screen.findByRole('button', { name: 'Select “Send the deck”' }));
    fireEvent.click(screen.getByRole('button', { name: 'Select “Book the room”' }), {
      shiftKey: true,
    });
    const bar = screen.getByRole('toolbar', { name: 'Selected action items' });
    expect(within(bar).getByText('2 selected')).toBeDefined();
    fireEvent.click(within(bar).getByRole('button', { name: 'Assign' }));
    fireEvent.click(await screen.findByRole('option', { name: /Maya Chen/ }));

    await waitFor(() => {
      expect(commands.bulkUpdate).toHaveBeenCalledWith([late.id, soon.id], {
        field: 'assignee',
        personId: MAYA,
      });
    });
  });

  test('⌘A selects every listed item and Esc clears it', async () => {
    renderScreen();
    await screen.findByText('Book the room');

    fireEvent.keyDown(window, { key: 'a', metaKey: true });
    expect(await screen.findByText('2 selected')).toBeDefined();
    fireEvent.keyDown(window, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByText('2 selected')).toBeNull();
    });
  });
});
