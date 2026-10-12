import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/player';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { usePlaybackStore } from '../store';
import type { MeetingId } from '../types';
import { MeetingPlayer } from './meeting-player';

const player = vi.hoisted(() => ({
  media: vi.fn<typeof Generated.commands.media>(),
  download: vi.fn<typeof Generated.commands.download>(),
}));
vi.mock('@/lib/ipc/generated/player', () => ({ commands: player }));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => `asset://${path}` }));

const MEETING = 'meeting-standup' as MeetingId;

function renderPlayer() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <MeetingPlayer meetingId={MEETING} bands={[{ start: 0, end: 1_000, color: 2 }]} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  player.media.mockReset();
  usePlaybackStore.setState({ meetingId: null, controller: null });
});

describe('MeetingPlayer', () => {
  it('shows nothing for a meeting without a recording', async () => {
    player.media.mockResolvedValue({ status: 'ok', data: { files: [] } });

    const { container } = renderPlayer();

    await vi.waitFor(() => {
      expect(player.media).toHaveBeenCalledWith(MEETING);
    });
    expect(container.querySelector('[role="group"]')).toBeNull();
  });

  it('docks the player over the recording and makes it the page player', async () => {
    player.media.mockResolvedValue({
      status: 'ok',
      data: {
        files: [
          { source: 'microphone', path: '/rec/m/mic.m4a' },
          { source: 'system', path: '/rec/m/system.m4a' },
        ],
      },
    });

    const { container } = renderPlayer();

    expect(await screen.findByRole('group', { name: 'Recording' })).toBeTruthy();
    expect(screen.getByRole('slider', { name: 'Playback position' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Audio track' }).textContent).toContain('Me');
    expect(container.querySelector('audio')?.getAttribute('src')).toBe('asset:///rec/m/mic.m4a');
    expect(usePlaybackStore.getState().meetingId).toBe(MEETING);
  });

  it('skips with the transport buttons, from the keyboard too', async () => {
    player.media.mockResolvedValue({
      status: 'ok',
      data: { files: [{ source: 'mixed', path: '/rec/m/audio.mp4' }] },
    });
    renderPlayer();
    await screen.findByRole('group', { name: 'Recording' });
    const controller = usePlaybackStore.getState().controller;
    if (controller === null) throw new Error('the player did not register its controller');
    const skip = vi.spyOn(controller, 'skip');

    fireEvent.click(screen.getByRole('button', { name: 'Forward 15 seconds' }));
    fireEvent.keyDown(document.body, { key: 'ArrowLeft' });

    expect(skip.mock.calls).toEqual([[15_000], [-15_000]]);
    expect(screen.queryByRole('button', { name: 'Audio track' })).toBeNull();
  });
});
