import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { usePlaybackStore } from '../store';
import type { MeetingId } from '../types';
import { usePlaybackListener, usePlaybackValue } from './use-playback';

const MEETING = 'meeting-a' as MeetingId;

beforeEach(() => {
  usePlaybackStore.setState({ meetingId: MEETING, time: 0, playing: false, engaged: false });
});

describe('usePlaybackValue', () => {
  it('is null until the recording is played or moved, then follows the position', () => {
    const { result } = renderHook(() => usePlaybackValue(MEETING, (ms) => Math.floor(ms / 1000)));
    expect(result.current).toBeNull();

    act(() => {
      usePlaybackStore.setState({ time: 4_200 });
    });

    expect(result.current).toBe(4);
  });
});

describe('usePlaybackListener', () => {
  it('reports the position while playing, not a seek while paused (a link it already shows)', () => {
    const onTime = vi.fn<(ms: number) => void>();
    renderHook(() => {
      usePlaybackListener(MEETING, onTime);
    });

    act(() => {
      usePlaybackStore.setState({ time: 54_000 });
    });
    act(() => {
      usePlaybackStore.setState({ playing: true, time: 54_100 });
    });

    expect(onTime.mock.calls).toEqual([[54_100]]);
  });
});
