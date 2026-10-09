import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/example';
import { createQueryClient } from '@/lib/ipc';

import { copy } from '../copy';
import type { ExampleNote, ExampleNoteId } from '../types';
import { ExampleNotes } from './example-notes';

// The generated bindings, mocked with their real types: a Rust signature
// change makes this file fail to compile instead of silently passing.
const bindings = vi.hoisted(() => {
  const noteAdded: { handler?: EventCallback<Generated.NoteAdded> } = {};
  const commands = {
    list: vi.fn<typeof Generated.commands.list>(),
    add: vi.fn<typeof Generated.commands.add>(),
  } satisfies typeof Generated.commands;
  const listen = vi.fn((handler: EventCallback<Generated.NoteAdded>) => {
    noteAdded.handler = handler;
    return Promise.resolve(vi.fn());
  });
  return { commands, events: { noteAdded: { listen } }, noteAdded };
});
vi.mock('@/lib/ipc/generated/example', () => ({
  commands: bindings.commands,
  events: bindings.events,
}));

function note(text: string): ExampleNote {
  return { id: `id-${text}` as ExampleNoteId, text, createdAt: '2026-10-08T09:00:00Z' };
}

function renderNotes() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ExampleNotes />
    </QueryClientProvider>,
  );
}

function submit(text: string) {
  fireEvent.change(screen.getByLabelText(copy.placeholder), { target: { value: text } });
  fireEvent.click(screen.getByRole('button', { name: copy.add }));
}

describe('ExampleNotes', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    bindings.commands.list.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('shows the empty state, then a saved note, and clears the form', async () => {
    renderNotes();
    expect(await screen.findByText(copy.empty)).toBeDefined();
    bindings.commands.add.mockResolvedValue({ status: 'ok', data: note('buy milk') });

    submit('buy milk');

    expect(await screen.findByText('buy milk')).toBeDefined();
    expect(bindings.commands.add).toHaveBeenCalledWith('buy milk');
    expect(screen.getByLabelText<HTMLInputElement>(copy.placeholder).value).toBe('');
  });

  it('shows the message of a rejected note and keeps the text', async () => {
    renderNotes();
    bindings.commands.add.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'a note cannot be empty' },
    });

    submit('   ');

    expect((await screen.findByRole('alert')).textContent).toBe('a note cannot be empty');
    expect(screen.getByLabelText<HTMLInputElement>(copy.placeholder).value).toBe('   ');
  });

  it('highlights the note that Rust just announced', async () => {
    bindings.commands.list.mockResolvedValue({
      status: 'ok',
      data: [note('first'), note('second')],
    });
    renderNotes();
    const second = await screen.findByText('second');

    act(() => {
      const payload = { note: note('second') };
      bindings.noteAdded.handler?.({ event: 'plugin:example:note-added', id: 1, payload });
    });

    expect(second.className).toContain('bg-accent');
    expect(screen.getByText('first').className).not.toContain('bg-accent');
  });
});
