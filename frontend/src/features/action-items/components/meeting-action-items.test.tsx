import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/action-items';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { todoOf } from '../notes-sync';
import type { ActionItemId, ActionItemView } from '../types';
import { MeetingActionItems } from './meeting-action-items';

const commands = vi.hoisted(() => ({
  list: vi.fn<typeof Generated.commands.list>(),
  update: vi.fn<typeof Generated.commands.update>(),
  filterOptions: vi.fn<typeof Generated.commands.filterOptions>(),
}));
vi.mock('@/lib/ipc/generated/action-items', () => ({ commands, events: {} }));

const MEETING = asMeetingId('meeting-fixture-standup');

function item(index: number, status: 'open' | 'done' = 'open'): ActionItemView {
  const now = new Date().toISOString();
  return {
    id: `item-${String(index)}` as ActionItemId,
    meetingId: MEETING,
    title: `Item ${String(index)}`,
    description: '',
    assigneePersonId: null,
    dueAt: null,
    status,
    priority: null,
    sourceSegmentId: null,
    completedAt: status === 'done' ? now : null,
    createdBy: 'ai',
    createdAt: now,
    updatedAt: now,
    meetingTitle: 'Eng standup',
    meetingHeldAt: now,
    assigneeName: null,
    assigneeIsMe: false,
    sourceStart: null,
  };
}

function renderBlock() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <MeetingActionItems meetingId={MEETING} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe("a meeting's action items on its Notes tab", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.filterOptions.mockResolvedValue({
      status: 'ok',
      data: { people: [], companies: [], tags: [], folders: [] },
    });
  });

  test('lists open items first, five at a time, and ticking one completes it', async () => {
    const items = [item(1, 'done'), ...[2, 3, 4, 5, 6, 7].map((index) => item(index))];
    commands.list.mockResolvedValue({ status: 'ok', data: items });
    commands.update.mockResolvedValue({ status: 'ok', data: { ...item(2), status: 'done' } });
    renderBlock();

    expect(await screen.findByRole('heading', { name: /Action items/ })).toBeDefined();
    expect(screen.getByText('6 open')).toBeDefined();
    expect(screen.queryByText('Item 1')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Show all 7' }));
    expect(screen.getByText('Item 1')).toBeDefined();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Mark “Item 2” done' }));

    await waitFor(() => {
      expect(commands.update).toHaveBeenCalledWith('item-2', { field: 'status', status: 'done' });
    });
    expect(commands.list).toHaveBeenCalledWith(
      expect.objectContaining({ meetingId: MEETING, status: 'any', assignee: { kind: 'anyone' } }),
    );
  });

  test('a meeting without items shows nothing', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [] });
    renderBlock();

    await waitFor(() => {
      expect(commands.list).toHaveBeenCalled();
    });
    expect(screen.queryByRole('heading')).toBeNull();
  });

  test('items read as todo blocks for the enhanced notes', () => {
    expect(todoOf(item(3, 'done'))).toEqual({
      id: 'item-3',
      text: 'Item 3',
      checked: true,
      assignee: null,
      dueAt: null,
    });
  });
});
