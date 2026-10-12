import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { AskMessage } from '@/lib/ipc/generated/ask';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { Recipe, RecipeId } from '@/lib/ipc/generated/recipes';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { AskPanel } from './ask-panel';

/** A stand-in for Tauri's IPC channel: the mock command calls `onmessage`. */
const { FakeChannel } = vi.hoisted(() => ({
  FakeChannel: class {
    onmessage: (event: { kind: 'text'; delta: string }) => void = () => undefined;
  },
}));
type FakeChannel = InstanceType<typeof FakeChannel>;
vi.mock('@tauri-apps/api/core', () => ({ Channel: FakeChannel }));

const ask = vi.hoisted(() => ({
  thread: vi.fn(),
  ask: vi.fn(),
  preview: vi.fn(),
  clear: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/ask', () => ({ commands: ask }));
const recipes = vi.hoisted(() => ({ list: vi.fn() }));
vi.mock('@/lib/ipc/generated/recipes', () => ({ commands: recipes }));

const MEETING = 'meeting-1' as MeetingId;

function recipe(slug: string, name: string, kind: Recipe['kind'] = 'single'): Recipe {
  return {
    id: `id-${slug}` as RecipeId,
    slug,
    name,
    description: '',
    prompt: name,
    kind,
    builtIn: true,
    createdAt: '2026-10-12T09:00:00Z',
    updatedAt: '2026-10-12T09:00:00Z',
  };
}

const ANSWER: AskMessage = {
  id: 'answer-1' as AskMessage['id'],
  meetingId: MEETING,
  role: 'assistant',
  parts: [
    { text: 'They have 220 technicians', citations: [{ segmentId: 's-3' as never, start: 9000 }] },
    { text: '.', citations: [] },
  ],
  recipe: null,
  createdAt: '2026-10-12T09:00:01Z',
};

function renderPanel() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <AskPanel meetingId={MEETING} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

function bar() {
  return screen.getByRole('textbox', { name: /ask a question/i });
}

beforeEach(() => {
  vi.clearAllMocks();
  ask.thread.mockResolvedValue({ status: 'ok', data: [] });
  recipes.list.mockResolvedValue({
    status: 'ok',
    data: [
      recipe('write-follow-up-email', 'Write follow-up email'),
      recipe('weekly-recap', 'Weekly recap', 'multi'),
    ],
  });
});

describe('AskPanel', () => {
  it('streams an answer and shows its citations as numbered chips', async () => {
    ask.ask.mockImplementation((_m: MeetingId, _r: unknown, channel: FakeChannel) => {
      channel.onmessage({ kind: 'text', delta: 'They have 220 [s3' });
      ask.thread.mockResolvedValue({ status: 'ok', data: [ANSWER] });
      return Promise.resolve({ status: 'ok', data: ANSWER });
    });
    renderPanel();

    fireEvent.change(bar(), { target: { value: 'How big is the team?' } });
    fireEvent.keyDown(bar(), { key: 'Enter' });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Source 1' })).toBeTruthy();
    });
    expect(ask.ask).toHaveBeenCalledWith(
      MEETING,
      { question: 'How big is the team?', recipe: null },
      expect.anything(),
    );
    expect(screen.getByText('They have 220 technicians')).toBeTruthy();
  });

  it('lists only runnable recipes after / and sends the chosen one', async () => {
    ask.ask.mockResolvedValue({ status: 'ok', data: ANSWER });
    renderPanel();
    await screen.findByText('Write follow-up email');

    fireEvent.change(bar(), { target: { value: '/' } });
    const menu = screen.getByRole('listbox', { name: 'Recipes' });
    expect(within(menu).queryByText('Weekly recap')).toBeNull();
    fireEvent.keyDown(bar(), { key: 'Enter' });
    fireEvent.keyDown(bar(), { key: 'Enter' });

    await waitFor(() => {
      expect(ask.ask).toHaveBeenCalled();
    });
    expect(ask.ask.mock.calls[0]?.[1]).toEqual({
      question: '',
      recipe: 'id-write-follow-up-email',
    });
  });
});
