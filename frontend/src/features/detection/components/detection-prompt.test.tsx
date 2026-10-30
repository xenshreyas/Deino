import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/detection';
import { createQueryClient } from '@/lib/ipc';

import { asBundleId } from '../types';
import { DetectionPrompt } from './detection-prompt';

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
    current: vi.fn<typeof Generated.commands.current>(),
    takeNotes: vi.fn<typeof Generated.commands.takeNotes>(),
    dismiss: vi.fn<typeof Generated.commands.dismiss>(),
    ignoreApp: vi.fn<typeof Generated.commands.ignoreApp>(),
    settings: vi.fn<typeof Generated.commands.settings>(),
  } satisfies typeof Generated.commands;
  const events = {
    meetingDetected: { listen: listenFor('meetingDetected') },
    meetingEnded: { listen: listenFor('meetingEnded') },
    promptClosed: { listen: listenFor('promptClosed') },
    recordingRequested: { listen: listenFor('recordingRequested') },
  };
  return { commands, events, handlers };
});
vi.mock('@/lib/ipc/generated/detection', () => ({
  commands: bindings.commands,
  events: bindings.events,
}));

function detection(overrides: Partial<Generated.Detection> = {}): Generated.Detection {
  return {
    id: 1,
    app: { bundleId: asBundleId('us.zoom.xos'), name: 'zoom.us' },
    kind: 'meeting',
    platform: 'zoom',
    event: null,
    detectedAt: '2026-10-09T10:00:00Z',
    autoStartAt: null,
    ...overrides,
  };
}

/** Delivers a Rust event to the prompt, as Tauri would. */
function deliver(name: keyof Handlers, payload: unknown) {
  act(() => {
    bindings.handlers[name]?.({ event: name, id: 0, payload });
  });
}

function renderPrompt() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <DetectionPrompt />
    </QueryClientProvider>,
  );
}

/** Opens the caret menu beside Take notes (Radix opens on pointer down). */
async function openIgnoreMenu(): Promise<void> {
  fireEvent.pointerDown(await screen.findByRole('button', { name: 'More ways to ignore' }), {
    button: 0,
    ctrlKey: false,
  });
}

describe('DetectionPrompt', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bindings.commands.current.mockResolvedValue({ status: 'ok', data: null });
    bindings.commands.takeNotes.mockResolvedValue({ status: 'ok', data: null });
    bindings.commands.dismiss.mockResolvedValue({ status: 'ok', data: null });
    bindings.commands.ignoreApp.mockResolvedValue({ status: 'ok', data: null });
  });

  it('shows the prompt already on screen when the window opens', async () => {
    bindings.commands.current.mockResolvedValue({ status: 'ok', data: detection() });
    renderPrompt();

    expect(await screen.findByRole('heading', { name: 'Meeting detected' })).toBeTruthy();
    expect(screen.getByText('zoom.us')).toBeTruthy();
  });

  it('titles huddles and calls, and names the linked calendar event', async () => {
    const event = {
      id: '01926b2c-0000-7000-8000-000000000001' as Generated.CalendarEventId,
      title: 'Design review',
      startsAt: '2026-10-09T09:58:00Z',
    };
    bindings.commands.current.mockResolvedValue({
      status: 'ok',
      data: detection({ kind: 'huddle', event }),
    });
    renderPrompt();

    // A calendar meeting is titled after its event; the kind titles the rest.
    expect(await screen.findByRole('heading', { name: 'Design review' })).toBeTruthy();

    deliver('meetingDetected', { detection: detection({ id: 2, kind: 'call' }) });
    expect(await screen.findByRole('heading', { name: 'Call detected' })).toBeTruthy();
  });

  it('take notes, ignore and always ignore answer the prompt on screen', async () => {
    bindings.commands.current.mockResolvedValue({ status: 'ok', data: detection({ id: 7 }) });
    renderPrompt();

    fireEvent.click(await screen.findByRole('button', { name: 'Take notes' }));
    await waitFor(() => {
      expect(screen.queryByRole('heading')).toBeNull();
    });
    deliver('meetingDetected', { detection: detection({ id: 8 }) });
    // Ignore sits beside Take notes; the permanent answer is in its caret menu.
    fireEvent.click(await screen.findByRole('button', { name: 'Ignore' }));
    await waitFor(() => {
      expect(screen.queryByRole('heading')).toBeNull();
    });
    deliver('meetingDetected', { detection: detection({ id: 10 }) });
    // The close button dismisses too.
    fireEvent.click(await screen.findByRole('button', { name: 'Close' }));
    await waitFor(() => {
      expect(screen.queryByRole('heading')).toBeNull();
    });
    deliver('meetingDetected', { detection: detection({ id: 9 }) });
    await openIgnoreMenu();
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Always ignore zoom.us' }));

    await waitFor(() => {
      expect(bindings.commands.ignoreApp).toHaveBeenCalledWith(9);
    });
    expect(bindings.commands.takeNotes).toHaveBeenCalledWith(7);
    expect(bindings.commands.dismiss).toHaveBeenCalledWith(8);
    expect(bindings.commands.dismiss).toHaveBeenCalledWith(10);
  });

  it('counts down to the auto-start and offers Don’t record instead of Ignore', async () => {
    const inTenSeconds = new Date(Date.now() + 10_000).toISOString();
    bindings.commands.current.mockResolvedValue({
      status: 'ok',
      data: detection({ autoStartAt: inTenSeconds }),
    });
    renderPrompt();

    expect(await screen.findByText(/Recording in 10s/)).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Ignore' })).toBeNull();
    fireEvent.click(await screen.findByRole('button', { name: 'Don’t record' }));

    await waitFor(() => {
      expect(bindings.commands.dismiss).toHaveBeenCalledWith(1);
    });
  });

  it('disappears when the meeting ends or the prompt closes in Rust', async () => {
    bindings.commands.current.mockResolvedValue({ status: 'ok', data: detection() });
    renderPrompt();
    await screen.findByRole('heading', { name: 'Meeting detected' });

    deliver('meetingEnded', { id: 1 });
    await waitFor(() => {
      expect(screen.queryByRole('heading')).toBeNull();
    });
    deliver('meetingDetected', { detection: detection({ id: 2 }) });
    await screen.findByRole('heading', { name: 'Meeting detected' });
    deliver('promptClosed', { id: 2 });

    await waitFor(() => {
      expect(screen.queryByRole('heading')).toBeNull();
    });
  });

  it('says so when an answer fails, and keeps the prompt', async () => {
    bindings.commands.current.mockResolvedValue({ status: 'ok', data: detection() });
    bindings.commands.takeNotes.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'could not start recording' },
    });
    renderPrompt();

    fireEvent.click(await screen.findByRole('button', { name: 'Take notes' }));

    expect((await screen.findByRole('alert')).textContent).toContain('That didn’t work');
    expect(screen.getByRole('heading', { name: 'Meeting detected' })).toBeTruthy();
  });
});
