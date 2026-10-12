import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Enhance from '@/lib/ipc/generated/enhance';
import type * as Generated from '@/lib/ipc/generated/notes-edit';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { TEST_MEETING, testView } from '../test-notes';
import { editCopy } from './copy';
import { EditBar } from './edit-bar';
import { undoEdit } from './hooks';
import { openEditBar, registerSelectionReader, useEditBarStore } from './store';
import type { EditProposal } from './types';

// The generated bindings, mocked with their real types.
const edits = vi.hoisted(() => ({
  propose: vi.fn<typeof Generated.commands.propose>(),
  accept: vi.fn<typeof Generated.commands.accept>(),
}));
vi.mock('@/lib/ipc/generated/notes-edit', () => ({ commands: edits }));
const enhance = vi.hoisted(() => ({ restore: vi.fn<typeof Enhance.commands.restore>() }));
vi.mock('@/lib/ipc/generated/enhance', () => ({ commands: enhance }));

function wrapper({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>{children}</TooltipProvider>
    </QueryClientProvider>
  );
}

const PROPOSAL: EditProposal = {
  meetingId: TEST_MEETING,
  baseVersion: 1,
  method: 'rename',
  occurrences: 2,
  doc: testView().doc,
  changes: [
    {
      change: 'changed',
      kind: 'bullet',
      section: 'Pricing',
      segments: [
        { change: 'removed', text: 'Niall' },
        { change: 'added', text: 'Nyal' },
        { change: 'same', text: ' owns pricing' },
      ],
    },
  ],
};

function ask(request: string) {
  const field = screen.getByRole('textbox', { name: editCopy.inputLabel });
  fireEvent.change(field, { target: { value: request } });
  fireEvent.submit(field);
}

describe('EditBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useEditBarStore.setState({ open: false, scope: [], reader: null });
    edits.propose.mockResolvedValue({ status: 'ok', data: PROPOSAL });
    edits.accept.mockResolvedValue({ status: 'ok', data: { version: 2, previousVersion: 1 } });
  });

  it('opens on the selection and proposes an edit scoped to it', async () => {
    const unregister = registerSelectionReader(() => [1, 2]);
    openEditBar();
    render(<EditBar meetingId={TEST_MEETING} version={1} />, { wrapper });

    expect(screen.getByText(editCopy.selection(2))).toBeTruthy();
    ask('Niall is spelled Nyal');

    await waitFor(() => {
      expect(edits.propose).toHaveBeenCalledWith({
        meetingId: TEST_MEETING,
        version: 1,
        instruction: 'Niall is spelled Nyal',
        selection: [1, 2],
      });
    });
    unregister();
  });

  it('shows the diff inline and side by side, and accepting stores it and closes', async () => {
    openEditBar();
    render(<EditBar meetingId={TEST_MEETING} version={1} />, { wrapper });
    ask('Niall is spelled Nyal');

    const preview = await screen.findByRole('list', { name: editCopy.previewTitle });
    expect(within(preview).getByText('Niall').tagName).toBe('DEL');
    expect(within(preview).getByText('Nyal').tagName).toBe('INS');
    expect(screen.getByText(`· ${editCopy.renamed(2)}`)).toBeTruthy();
    fireEvent.click(screen.getByRole('radio', { name: editCopy.split }));
    expect(screen.getByLabelText(editCopy.before).textContent).toContain('Niall owns pricing');
    expect(screen.getByLabelText(editCopy.after).textContent).toContain('Nyal owns pricing');

    fireEvent.click(screen.getByRole('button', { name: editCopy.accept }));

    await waitFor(() => {
      expect(edits.accept).toHaveBeenCalledWith(TEST_MEETING, 1, PROPOSAL.doc);
    });
    await waitFor(() => {
      expect(useEditBarStore.getState().open).toBe(false);
    });
  });

  it('rejecting drops the proposal and keeps the request open', async () => {
    openEditBar();
    render(<EditBar meetingId={TEST_MEETING} version={1} />, { wrapper });
    ask('Niall is spelled Nyal');
    fireEvent.click(await screen.findByRole('button', { name: editCopy.reject }));

    await waitFor(() => {
      expect(screen.queryByRole('list', { name: editCopy.previewTitle })).toBeNull();
    });
    expect(edits.accept).not.toHaveBeenCalled();
    expect(screen.getByRole('textbox', { name: editCopy.inputLabel })).toBeTruthy();
  });

  it('undo restores the version the edit was made from', async () => {
    enhance.restore.mockResolvedValue({ status: 'ok', data: testView() });

    await undoEdit(TEST_MEETING, { version: 2, previousVersion: 1 });

    expect(enhance.restore).toHaveBeenCalledWith(TEST_MEETING, 1);
  });
});
