import type { EventCallback } from '@tauri-apps/api/event';
import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/transcription';

import { copy } from '../copy';
import { LiveTranscriptLine } from './live-transcript-line';

const live = vi.hoisted(() => {
  const state: { handler?: EventCallback<Generated.LiveTranscript> } = {};
  const listen = vi.fn((handler: EventCallback<Generated.LiveTranscript>) => {
    state.handler = handler;
    return Promise.resolve(vi.fn());
  });
  return { state, events: { liveTranscript: { listen } } };
});
vi.mock('@/lib/ipc/generated/transcription', () => ({ events: live.events }));

function announce(committed: string, tentative = '') {
  act(() => {
    live.state.handler?.({
      event: 'plugin:transcription:live-transcript',
      id: 1,
      payload: { lane: 'mixed', committed, tentative, startSeconds: 3.2 },
    });
  });
}

describe('LiveTranscriptLine', () => {
  it('shows the segment being spoken and hides once it closes', () => {
    render(<LiveTranscriptLine />);
    expect(screen.queryByLabelText(copy.live)).toBeNull();

    announce('So the plan is', 'to ship');
    expect(screen.getByLabelText(copy.live).textContent).toBe('So the plan is to ship');

    announce('');
    expect(screen.queryByLabelText(copy.live)).toBeNull();
  });
});
