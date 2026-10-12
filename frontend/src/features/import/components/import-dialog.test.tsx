import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/import';
import { createQueryClient } from '@/lib/ipc';

import type { ImportCandidate, MeetingId } from '../types';
import { ImportDialog } from './import-dialog';

const commands = vi.hoisted(
  () =>
    ({
      pickFile: vi.fn<typeof Generated.commands.pickFile>(),
      check: vi.fn<typeof Generated.commands.check>(),
      start: vi.fn<typeof Generated.commands.start>(),
      watchedFolders: vi.fn<typeof Generated.commands.watchedFolders>(),
      addWatchedFolder: vi.fn<typeof Generated.commands.addWatchedFolder>(),
      removeWatchedFolder: vi.fn<typeof Generated.commands.removeWatchedFolder>(),
      setMoveToProcessed: vi.fn<typeof Generated.commands.setMoveToProcessed>(),
      watchFailures: vi.fn<typeof Generated.commands.watchFailures>(),
      retryFailure: vi.fn<typeof Generated.commands.retryFailure>(),
      dismissFailure: vi.fn<typeof Generated.commands.dismissFailure>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/import', () => ({ commands }));

const navigate = vi.hoisted(() => vi.fn());
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => navigate }));
vi.mock('@/features/event-links', () => ({
  MeetingEventChip: ({ meetingId }: { meetingId: string }) => <span>event chip {meetingId}</span>,
}));

const MEETING = 'meeting-imported' as MeetingId;

const video: ImportCandidate = {
  path: '/Users/me/Downloads/Customer call.mp4',
  fileName: 'Customer call.mp4',
  kind: 'video',
};

function renderDialog(filePath: string | null) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ImportDialog open filePath={filePath} onOpenChange={vi.fn()} />
    </QueryClientProvider>,
  );
}

describe('the import dialog', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.pickFile.mockResolvedValue({ status: 'ok', data: video });
    commands.check.mockResolvedValue({ status: 'ok', data: video });
    commands.start.mockResolvedValue({
      status: 'ok',
      data: { meetingId: MEETING, title: 'Customer call', duration: 3_312, hasVideo: true },
    });
  });

  test('a picked file imports with its title, then offers the event chip and the meeting', async () => {
    renderDialog(null);

    fireEvent.click(screen.getByRole('button', { name: 'Choose file…' }));
    expect(await screen.findByText('Customer call.mp4')).toBeDefined();
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Acme renewal' } });
    fireEvent.click(screen.getByRole('button', { name: 'Import' }));

    expect(await screen.findByText(`event chip ${MEETING}`)).toBeDefined();
    expect(commands.start).toHaveBeenCalledWith(video.path, 'Acme renewal');
    fireEvent.click(screen.getByRole('button', { name: 'Open meeting' }));
    expect(navigate).toHaveBeenCalledWith({
      to: '/meetings/$meetingId',
      params: { meetingId: MEETING },
    });
  });

  test('a dropped file is checked first and imports under its own name', async () => {
    renderDialog(video.path);

    expect(await screen.findByText('Customer call.mp4')).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: 'Import' }));

    await waitFor(() => {
      expect(commands.start).toHaveBeenCalledWith(video.path, null);
    });
    expect(commands.check).toHaveBeenCalledWith(video.path);
  });

  test('a file Rust refuses shows the reason inline', async () => {
    commands.check.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'this kind of file cannot be imported' },
    });
    renderDialog('/Users/me/notes.txt');

    expect((await screen.findByRole('alert')).textContent).toContain('cannot be imported');
    expect(screen.getByRole('button', { name: 'Choose file…' })).toBeDefined();
  });
});
