import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/recording';
import { createQueryClient } from '@/lib/ipc';

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
    resumeMeeting: vi.fn<typeof Generated.commands.resumeMeeting>(),
    setMicrophone: vi.fn<typeof Generated.commands.setMicrophone>(),
    setSystemAudio: vi.fn<typeof Generated.commands.setSystemAudio>(),
    microphones: vi.fn<typeof Generated.commands.microphones>(),
    keepRecording: vi.fn<typeof Generated.commands.keepRecording>(),
    answerSplit: vi.fn<typeof Generated.commands.answerSplit>(),
    autoStop: vi.fn<typeof Generated.commands.autoStop>(),
    openLiveMeeting: vi.fn<typeof Generated.commands.openLiveMeeting>(),
    setPanelRegion: vi.fn<typeof Generated.commands.setPanelRegion>(),
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
