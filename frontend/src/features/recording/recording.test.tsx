import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/recording';
import { createQueryClient } from '@/lib/ipc';

import { LiveTranscript } from './components/live-transcript';
import { RecordingControls } from './components/recording-controls';
import { IDLE } from './queries';

type Handlers = Partial<Record<keyof typeof Generated.events, EventCallback<unknown>>>;

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => {
  const handlers: Handlers = {};
  const listenFor = (name: keyof Handlers) =>
    vi.fn((handler: EventCallback<unknown>) => {
      handlers[name] = handler;
      return Promise.resolve(vi.fn());
    });
  const commands = {
    start: vi.fn<typeof Generated.commands.start>(),
    stop: vi.fn<typeof Generated.commands.stop>(),
    pause: vi.fn<typeof Generated.commands.pause>(),
    resume: vi.fn<typeof Generated.commands.resume>(),
    state: vi.fn<typeof Generated.commands.state>(),
    transcript: vi.fn<typeof Generated.commands.transcript>(),
  } satisfies typeof Generated.commands;
  const events = {
    health: { listen: listenFor('health') },
    liveSegment: { listen: listenFor('liveSegment') },
    recordingState: { listen: listenFor('recordingState') },
  };
  return { commands, events, handlers };
});
vi.mock('@/lib/ipc/generated/recording', () => ({
  commands: bindings.commands,
  events: bindings.events,
}));

const MEETING = asMeetingId('meeting-live');

const RECORDING: Generated.RecordingSnapshot = {
  ...IDLE,
  phase: 'recording',
  meetingId: MEETING,
  title: 'Standup',
  startedAt: new Date().toISOString(),
  echo: 'cancel',
};

function renderWithClient(ui: React.ReactElement) {
  render(<QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>);
}

function segment(text: string): Generated.Segment {
  return {
    id: 'segment-1' as Generated.SegmentId,
    meetingId: MEETING,
    channel: 'system',
    speakerId: null,
    start: 0,
    end: 2_000,
    text,
    words: [],
    isDeleted: false,
    editedAt: null,
  };
}

describe('RecordingControls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bindings.commands.start.mockResolvedValue({ status: 'ok', data: MEETING });
    bindings.commands.stop.mockResolvedValue({ status: 'ok', data: MEETING });
  });

  it('starts a recording in Rust from the idle bar', async () => {
    bindings.commands.state.mockResolvedValue({ status: 'ok', data: IDLE });
    renderWithClient(<RecordingControls />);

    fireEvent.click(await screen.findByRole('button', { name: /Start recording/ }));

    await waitFor(() => {
      expect(bindings.commands.start).toHaveBeenCalledWith(null);
    });
  });

  it('pauses and stops a running recording', async () => {
    bindings.commands.state.mockResolvedValue({ status: 'ok', data: RECORDING });
    bindings.commands.pause.mockResolvedValue({
      status: 'ok',
      data: { ...RECORDING, phase: 'paused', pausedAt: new Date().toISOString() },
    });
    renderWithClient(<RecordingControls />);

    fireEvent.click(await screen.findByRole('button', { name: /Pause/ }));
    expect(await screen.findByRole('button', { name: /Resume/ })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Stop/ }));

    await waitFor(() => {
      expect(bindings.commands.stop).toHaveBeenCalledOnce();
    });
  });
});

describe('LiveTranscript', () => {
  it('shows saved lines, then grows them with live updates, labelled by channel', async () => {
    bindings.commands.transcript.mockResolvedValue({ status: 'ok', data: [segment('Hello')] });
    renderWithClient(<LiveTranscript meetingId={MEETING} />);
    expect(await screen.findByText('Hello')).toBeTruthy();
    expect(screen.getByText('Them')).toBeTruthy();

    const update: Generated.SegmentUpdate = {
      meetingId: MEETING,
      segmentId: 'segment-2' as Generated.SegmentId,
      channel: 'mic',
      start: 2_500,
      end: 2_500,
      text: 'Hi, shall we',
      state: 'open',
    };
    act(() => {
      bindings.handlers.liveSegment?.({ event: 'live-segment', id: 0, payload: { update } });
    });

    expect(await screen.findByText('Hi, shall we')).toBeTruthy();
    expect(screen.getByText('Me')).toBeTruthy();
  });
});
