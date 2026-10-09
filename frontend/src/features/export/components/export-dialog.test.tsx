import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/export';
import { createQueryClient } from '@/lib/ipc';

import { ExportDialog } from './export-dialog';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const exporting = vi.hoisted(() => ({
  preview: vi.fn<typeof Generated.commands.preview>(),
  save: vi.fn<typeof Generated.commands.save>(),
}));
vi.mock('@/lib/ipc/generated/export', () => ({ commands: exporting, events: {} }));

const toasts = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));
vi.mock('sonner', () => ({ toast: toasts }));

const MEETING = asMeetingId('meeting-fixture-standup');

function preview(format: Generated.ExportFormat): Generated.ExportPreview {
  const html = format === 'pdf' || format === 'docx';
  return {
    format,
    kind: html ? 'html' : 'text',
    content: html ? '<h1>Eng standup</h1>' : `# Eng standup (${format})`,
    fileName: `2026-10-08 Eng standup.${format === 'markdown' ? 'md' : format}`,
  };
}

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

function renderDialog(onOpenChange = vi.fn()) {
  render(<ExportDialog meetingId={MEETING} open onOpenChange={onOpenChange} />, { wrapper });
  return onOpenChange;
}

describe('ExportDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    exporting.preview.mockImplementation((_, format) =>
      Promise.resolve({ status: 'ok', data: preview(format) }),
    );
  });

  it('previews Markdown without the transcript by default', async () => {
    renderDialog();

    expect(await screen.findByText('# Eng standup (markdown)')).toBeTruthy();
    expect(screen.getByText('2026-10-08 Eng standup.md')).toBeTruthy();
    const sections = exporting.preview.mock.calls[0]?.[2];
    expect(sections).toMatchObject({ notes: true, transcript: false });
  });

  it('switching to PDF renders the document in a sandboxed frame', async () => {
    renderDialog();
    await screen.findByText('# Eng standup (markdown)');

    fireEvent.click(screen.getByRole('radio', { name: /PDF/ }));

    const frame = await screen.findByTitle('Document preview');
    expect(frame.getAttribute('sandbox')).toBe('');
    expect(frame.getAttribute('srcdoc')).toContain('<h1>Eng standup</h1>');
  });

  it('turning a section on re-renders the preview with it', async () => {
    renderDialog();
    await screen.findByText('# Eng standup (markdown)');

    fireEvent.click(screen.getByRole('button', { name: 'Transcript' }));

    await waitFor(() => {
      expect(exporting.preview.mock.calls.at(-1)?.[2]).toMatchObject({ transcript: true });
    });
  });

  it('formats with a fixed content explain why the sections are off', async () => {
    renderDialog();
    await screen.findByText('# Eng standup (markdown)');

    fireEvent.click(screen.getByRole('radio', { name: /SRT/ }));

    expect(await screen.findByText('SRT is the transcript with speakers.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Notes' }).hasAttribute('disabled')).toBe(true);
  });

  it('exporting saves the chosen format and closes with a confirmation', async () => {
    exporting.save.mockResolvedValue({ status: 'ok', data: { fileName: 'standup.md' } });
    const onOpenChange = renderDialog();
    await screen.findByText('# Eng standup (markdown)');

    fireEvent.click(screen.getByRole('button', { name: 'Export…' }));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
    expect(exporting.save.mock.calls[0]?.[1]).toBe('markdown');
    expect(toasts.success).toHaveBeenCalledWith('Saved standup.md');
  });

  it('a cancelled save dialog keeps the export dialog open', async () => {
    exporting.save.mockResolvedValue({ status: 'ok', data: null });
    const onOpenChange = renderDialog();
    await screen.findByText('# Eng standup (markdown)');

    fireEvent.click(screen.getByRole('button', { name: 'Export…' }));

    await waitFor(() => {
      expect(exporting.save).toHaveBeenCalled();
    });
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(toasts.success).not.toHaveBeenCalled();
  });
});
