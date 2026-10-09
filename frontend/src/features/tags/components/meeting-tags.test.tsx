import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/tags';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { MeetingId, Tag, TagId } from '../types';
import { MeetingTags } from './meeting-tags';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      create: vi.fn<typeof Generated.commands.create>(),
      update: vi.fn<typeof Generated.commands.update>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      tagsOf: vi.fn<typeof Generated.commands.tagsOf>(),
      links: vi.fn<typeof Generated.commands.links>(),
      apply: vi.fn<typeof Generated.commands.apply>(),
      remove: vi.fn<typeof Generated.commands.remove>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/tags', () => ({ commands, events: {} }));

const MEETING = 'meeting-1' as MeetingId;
const AT = '2026-10-09T09:00:00.000Z';

function tag(name: string, color: number): Tag {
  return {
    id: `tag-${name}` as TagId,
    name,
    color,
    description: '',
    autoApply: true,
    createdAt: AT,
    updatedAt: AT,
  };
}

const customer = tag('Customer', 1);
const hiring = tag('Hiring', 2);

function renderTags() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <MeetingTags meetingId={MEETING} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

/** Opens the combobox from the "Add tag" chip. */
async function openPicker() {
  fireEvent.click(await screen.findByRole('button', { name: 'Add tag' }));
  return screen.findByPlaceholderText('Find or create a tag');
}

describe('the meeting header’s tags', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.list.mockResolvedValue({ status: 'ok', data: [customer, hiring] });
    commands.tagsOf.mockResolvedValue({
      status: 'ok',
      data: [{ tag: customer, appliedBy: 'ai', appliedAt: AT }],
    });
    commands.apply.mockResolvedValue({ status: 'ok', data: null });
    commands.remove.mockResolvedValue({ status: 'ok', data: null });
  });

  test('shows each tag as a pill, marking the ones Deino applied', async () => {
    renderTags();

    expect(await screen.findByText('Customer')).toBeDefined();
    expect(screen.getByRole('img', { name: 'Added by Deino' })).toBeDefined();
  });

  test('choosing a tag in the combobox applies it; choosing an applied one removes it', async () => {
    renderTags();
    await openPicker();

    fireEvent.click(await screen.findByRole('option', { name: 'Hiring' }));
    await waitFor(() => {
      expect(commands.apply).toHaveBeenCalledWith(MEETING, hiring.id);
    });

    fireEvent.click(screen.getByRole('option', { name: 'Customer' }));
    await waitFor(() => {
      expect(commands.remove).toHaveBeenCalledWith(MEETING, customer.id);
    });
  });

  test('typing a new name offers to create the tag, then puts it on the meeting', async () => {
    const created = tag('Q4', 3);
    commands.create.mockResolvedValue({ status: 'ok', data: created });
    renderTags();
    const search = await openPicker();

    fireEvent.change(search, { target: { value: 'customer' } });
    expect(screen.queryByRole('option', { name: /Create/ })).toBeNull();
    fireEvent.change(search, { target: { value: ' Q4 ' } });
    fireEvent.click(await screen.findByRole('option', { name: 'Create “Q4”' }));

    await waitFor(() => {
      expect(commands.apply).toHaveBeenCalledWith(MEETING, created.id);
    });
    expect(commands.create).toHaveBeenCalledWith({ name: 'Q4', color: null, description: '' });
  });

  test('the pill’s × takes the tag off', async () => {
    renderTags();

    fireEvent.click(await screen.findByRole('button', { name: 'Remove Customer' }));

    await waitFor(() => {
      expect(commands.remove).toHaveBeenCalledWith(MEETING, customer.id);
    });
  });
});
