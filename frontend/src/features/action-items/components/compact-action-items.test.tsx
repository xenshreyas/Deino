import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/action-items';
import { createQueryClient } from '@/lib/ipc';

import type { ActionItemId, ActionItemView } from '../types';
import { CompactActionItems, ownerAndDue } from './compact-action-items';

const commands = vi.hoisted(() => ({
  list: vi.fn<typeof Generated.commands.list>(),
  update: vi.fn<typeof Generated.commands.update>(),
}));
vi.mock('@/lib/ipc/generated/action-items', () => ({ commands, events: {} }));

const MEETING = asMeetingId('meeting-fixture-standup');
const NOW = new Date(2026, 9, 14, 9, 0);

function item(index: number, extra: Partial<ActionItemView> = {}): ActionItemView {
  const at = NOW.toISOString();
  return {
    id: `item-${String(index)}` as ActionItemId,
    meetingId: MEETING,
    title: `Item ${String(index)}`,
    description: '',
    assigneePersonId: null,
    dueAt: null,
    status: 'open',
    priority: null,
    sourceSegmentId: null,
    completedAt: null,
    createdBy: 'ai',
    createdAt: at,
    updatedAt: at,
    meetingTitle: 'Eng standup',
    meetingHeldAt: at,
    assigneeName: null,
    assigneeIsMe: false,
    sourceStart: null,
    ...extra,
  };
}

describe("the context rail's action items", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  test('ticking an item completes it through the action items API', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [item(1), item(2, { status: 'done' })] });
    commands.update.mockResolvedValue({ status: 'ok', data: item(1, { status: 'done' }) });
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CompactActionItems meetingId={MEETING} />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole('region', { name: 'Action items' })).toBeDefined();
    expect(screen.getByText('1 open')).toBeDefined();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Mark “Item 1” done' }));

    await waitFor(() => {
      expect(commands.update).toHaveBeenCalledWith('item-1', { field: 'status', status: 'done' });
    });
  });

  test('the second line reads owner · due', () => {
    expect(ownerAndDue(item(1, { assigneeName: 'Ana', dueAt: '2026-10-15' }), NOW)).toBe(
      'Ana · Tomorrow',
    );
    expect(ownerAndDue(item(1, { assigneeIsMe: true, assigneeName: 'Sam' }), NOW)).toBe('Me');
    expect(ownerAndDue(item(1), NOW)).toBe('');
  });
});
