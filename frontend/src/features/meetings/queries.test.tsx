import { type QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/meetings';
import { createQueryClient, invalidateTables, IpcError } from '@/lib/ipc';

import { useMeetings, useRenameMeeting } from './queries';
import { asMeetingId, type Meeting } from './types';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      get: vi.fn<typeof Generated.commands.get>(),
      rename: vi.fn<typeof Generated.commands.rename>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/meetings', () => ({ commands }));

const standup: Meeting = {
  id: asMeetingId('meeting-1'),
  title: 'Standup',
  createdAt: '2026-10-08T09:00:00Z',
  updatedAt: '2026-10-08T09:00:00Z',
};

function renderWithClient<T>(hook: () => T): { result: { current: T }; client: QueryClient } {
  const client = createQueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const { result } = renderHook(hook, { wrapper });
  return { result, client };
}

describe('meetings queries', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.list.mockResolvedValue({ status: 'ok', data: [standup] });
  });

  it('loads the meeting list through the meetings plugin', async () => {
    const { result } = renderWithClient(useMeetings);

    await waitFor(() => {
      expect(result.current.data).toEqual([standup]);
    });
  });

  it('surfaces a command error as an IpcError with its code', async () => {
    commands.list.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'could not load or save meetings' },
    });

    const { result } = renderWithClient(useMeetings);

    await waitFor(() => {
      expect(result.current.error).toBeInstanceOf(IpcError);
    });
    expect(result.current.error?.code).toBe('internal');
  });

  it('shows a rename in the cached list without refetching', async () => {
    const renamed = { ...standup, title: 'Planning' };
    commands.rename.mockResolvedValue({ status: 'ok', data: renamed });
    const { result } = renderWithClient(() => ({
      list: useMeetings(),
      rename: useRenameMeeting(),
    }));
    await waitFor(() => {
      expect(result.current.list.data).toEqual([standup]);
    });

    await act(() => result.current.rename.mutateAsync({ id: standup.id, title: 'Planning' }));

    expect(commands.rename).toHaveBeenCalledWith(standup.id, 'Planning');
    await waitFor(() => {
      expect(result.current.list.data).toEqual([renamed]);
    });
    expect(commands.list).toHaveBeenCalledTimes(1);
  });

  it('refetches the list when Rust reports a change to meetings, and only then', async () => {
    const { result, client } = renderWithClient(useMeetings);
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    await act(() => invalidateTables(client, ['transcript_segments']));
    expect(commands.list).toHaveBeenCalledTimes(1);

    await act(() => invalidateTables(client, ['meetings']));
    expect(commands.list).toHaveBeenCalledTimes(2);
  });
});
