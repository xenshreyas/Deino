import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/import';
import { createQueryClient } from '@/lib/ipc';

import type { WatchedFileId, WatchedFolder, WatchedFolderId, WatchFailure } from '../types';
import { WatchedFoldersSettings } from './watched-folders-settings';

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

const FOLDER_ID = '0192a5f0-0000-7000-8000-00000000c001' as WatchedFolderId;
const FILE_ID = '0192a5f0-0000-7000-8000-00000000c101' as WatchedFileId;

const zoom: WatchedFolder = {
  id: FOLDER_ID,
  path: '/Users/me/Movies/Zoom',
  moveToProcessed: false,
  createdAt: '2026-10-12T09:00:00Z',
};

const silent: WatchFailure = {
  id: FILE_ID,
  folderId: FOLDER_ID,
  path: '/Users/me/Movies/Zoom/Screen Recording.mov',
  failure: 'no_audio',
  failedAt: '2026-10-12T10:00:00Z',
};

function renderSettings() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <WatchedFoldersSettings />
    </QueryClientProvider>,
  );
}

describe('Settings → Watched folders', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.watchedFolders.mockResolvedValue({ status: 'ok', data: [zoom] });
    commands.watchFailures.mockResolvedValue({ status: 'ok', data: [silent] });
    commands.setMoveToProcessed.mockResolvedValue({ status: 'ok', data: null });
    commands.removeWatchedFolder.mockResolvedValue({ status: 'ok', data: null });
    commands.retryFailure.mockResolvedValue({ status: 'ok', data: null });
    commands.dismissFailure.mockResolvedValue({ status: 'ok', data: null });
    commands.addWatchedFolder.mockResolvedValue({ status: 'ok', data: null });
  });

  test('lists each folder with its Move to processed switch, and toggles it', async () => {
    renderSettings();

    const folders = await screen.findByRole('group', { name: 'Folders' });
    expect(within(folders).getByText('/Users/me/Movies/Zoom')).toBeDefined();
    fireEvent.click(within(folders).getByRole('switch'));

    await waitFor(() => {
      expect(commands.setMoveToProcessed).toHaveBeenCalledWith(FOLDER_ID, true);
    });
  });

  test('removes a folder and adds one through the folder dialog', async () => {
    renderSettings();

    fireEvent.click(
      await screen.findByRole('button', { name: 'Stop watching /Users/me/Movies/Zoom' }),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Add folder…' }));

    await waitFor(() => {
      expect(commands.removeWatchedFolder).toHaveBeenCalledWith(FOLDER_ID);
      expect(commands.addWatchedFolder).toHaveBeenCalled();
    });
  });

  test('a failed file shows why, and can be retried or dismissed', async () => {
    renderSettings();

    const failures = await screen.findByRole('group', { name: 'Couldn’t import' });
    expect(within(failures).getByText('Screen Recording.mov')).toBeDefined();
    expect(within(failures).getByText('The file has no audio.')).toBeDefined();
    fireEvent.click(within(failures).getByRole('button', { name: 'Retry' }));
    await waitFor(() => {
      expect(commands.retryFailure).toHaveBeenCalledWith(FILE_ID);
    });
    fireEvent.click(within(failures).getByRole('button', { name: 'Dismiss' }));
    await waitFor(() => {
      expect(commands.dismissFailure).toHaveBeenCalledWith(FILE_ID);
    });
  });

  test('with nothing watched, the list explains what to add', async () => {
    commands.watchedFolders.mockResolvedValue({ status: 'ok', data: [] });
    commands.watchFailures.mockResolvedValue({ status: 'ok', data: [] });
    renderSettings();

    expect(await screen.findByText('No watched folders')).toBeDefined();
    expect(await screen.findByText('Nothing has failed.')).toBeDefined();
  });
});
