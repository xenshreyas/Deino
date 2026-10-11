import { QueryClientProvider } from '@tanstack/react-query';
import { Editor } from '@tiptap/core';
import { act, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/notes';
import { createQueryClient } from '@/lib/ipc';

import { AUTOSAVE_DELAY_MS } from './autosave';
import { Notepad } from './components/notepad';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      get: vi.fn<typeof Generated.commands.get>(),
      save: vi.fn<typeof Generated.commands.save>(),
      createScheduled: vi.fn<typeof Generated.commands.createScheduled>(),
      saveImage: vi.fn<typeof Generated.commands.saveImage>(),
      attachDroppedImage: vi.fn<typeof Generated.commands.attachDroppedImage>(),
      markdownToDoc: vi.fn<typeof Generated.commands.markdownToDoc>(),
      docToMarkdown: vi.fn<typeof Generated.commands.docToMarkdown>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/notes', () => ({ commands }));

const MEETING = asMeetingId('meeting-1');
const SAVED_AT = '2026-10-08T09:00:00Z';

function renderWithClient(ui: React.ReactElement) {
  return render(<QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>);
}

function editorElement(): HTMLElement {
  const element = document.querySelector<HTMLElement>('.note-content');
  if (element === null) throw new Error('no editor');
  return element;
}

describe('the notepad', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.get.mockImplementation((meetingId) =>
      Promise.resolve({
        status: 'ok',
        data: {
          meetingId,
          doc: {
            type: 'doc',
            content: [
              { type: 'paragraph', content: [{ type: 'text', text: 'agenda', marks: [] }] },
            ],
          },
          updatedAt: SAVED_AT,
          folder: '/data/meetings/meeting-1',
        },
      }),
    );
    commands.save.mockImplementation((meetingId) =>
      Promise.resolve({ status: 'ok', data: { meetingId, updatedAt: SAVED_AT } }),
    );
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('opens the saved notes and autosaves edits once typing pauses', async () => {
    renderWithClient(<Notepad meetingId={MEETING} />);
    await waitFor(() => {
      expect(editorElement().textContent).toBe('agenda');
    });

    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const editor: unknown = Reflect.get(editorElement(), 'editor');
    if (!(editor instanceof Editor)) throw new Error('TipTap did not expose its editor');
    act(() => {
      editor.commands.insertContentAt(editor.state.doc.content.size - 1, ' and more');
    });
    await act(() => vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS - 1));
    expect(commands.save).not.toHaveBeenCalled();

    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(commands.save).toHaveBeenCalledExactlyOnceWith(MEETING, {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'agenda and more', marks: [] }] },
      ],
    });
    await act(() => vi.advanceTimersByTimeAsync(0));
    expect(screen.getByRole('status').textContent).toBe('Saved');
  });
});
