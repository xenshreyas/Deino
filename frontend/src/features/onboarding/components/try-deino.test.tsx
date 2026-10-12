import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Onboarding from '@/lib/ipc/generated/onboarding';
import { createQueryClient } from '@/lib/ipc';

import { TryDeino } from './try-deino';

const bindings = vi.hoisted(() => ({
  importSample: vi.fn<typeof Onboarding.commands.importSample>(),
  navigate: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/onboarding', () => ({
  commands: { importSample: bindings.importSample },
}));
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => bindings.navigate }));
vi.mock('@/features/recording', () => ({
  recordingApi: { start: vi.fn(), stop: vi.fn() },
  useElapsed: () => 0,
  useRecordingState: () => ({ phase: 'idle' }),
}));

function renderRows() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TryDeino />
    </QueryClientProvider>,
  );
}

describe('Try Deino (UX-24: what used to end setup, now on Home)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('offers a test recording and the sample meeting', () => {
    renderRows();

    expect(screen.getByRole('list', { name: 'Try Deino' })).toBeTruthy();
    expect(screen.getByText('Record a test')).toBeTruthy();
    expect(screen.getByText('Open a sample meeting')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Record' })).toBeTruthy();
  });

  it('opens the sample meeting', async () => {
    bindings.importSample.mockResolvedValue({
      status: 'ok',
      data: 'meeting-sample' as Onboarding.MeetingId,
    });
    renderRows();

    fireEvent.click(screen.getByRole('button', { name: 'Open' }));

    await waitFor(() => {
      expect(bindings.navigate).toHaveBeenCalledWith({
        to: '/meetings/$meetingId',
        params: { meetingId: 'meeting-sample' },
      });
    });
  });
});
