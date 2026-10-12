import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { recordingSettingsCopy as copy } from '../copy';
import { defaultSettings } from '../settings-fixture';
import { AutoStopSettings, RecordingBehaviour } from './recording-behaviour';

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => ({
  recording: {
    microphones: vi.fn(() =>
      Promise.resolve({
        status: 'ok' as const,
        data: [{ name: 'MacBook Pro Microphone', isDefault: true, bluetooth: false }],
      }),
    ),
    state: vi.fn(() => new Promise(() => undefined)),
  },
  settings: {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands,
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));
vi.mock('@/lib/ipc/generated/recording', () => ({
  commands: bindings.recording,
  events: new Proxy({}, { get: () => ({ listen: bindings.listen }) }),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: bindings.settings,
  events: { settingsChanged: { listen: bindings.listen } },
}));

function renderSection() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RecordingBehaviour />
      <AutoStopSettings />
    </QueryClientProvider>,
  );
}

/** Opens "More options" under End automatically. */
async function openMoreOptions() {
  fireEvent.click(await screen.findByRole('button', { name: 'More options' }));
}

describe('Settings → Recording', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const settings = defaultSettings();
    bindings.settings.get.mockResolvedValue({ status: 'ok', data: settings });
    bindings.settings.patch.mockResolvedValue({ status: 'ok', data: settings });
  });

  it('hides the record panel with a merge patch', async () => {
    renderSection();

    fireEvent.click(await screen.findByRole('switch', { name: copy.panelTitle }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({ recording: { showPanel: false } });
    });
  });

  it('shows the microphone, the panel and auto-stop, nothing that defeats a default', async () => {
    renderSection();

    expect(await screen.findByRole('combobox', { name: copy.microphoneTitle })).toBeTruthy();
    expect(screen.getAllByRole('switch')).toHaveLength(2);
    expect(screen.queryByText(/headphones/i)).toBeNull();
    expect(screen.queryByText(/Save audio/i)).toBeNull();
  });

  it('keeps the calendar rule off by default and turns it on alone', async () => {
    renderSection();
    await openMoreOptions();
    const calendar = await screen.findByRole('switch', { name: copy.calendarTitle });

    expect(calendar.getAttribute('aria-checked')).toBe('false');
    fireEvent.click(calendar);

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        recording: {
          autoStop: { enabled: true, afterCalendarEnd: true, offerNextMeeting: true },
        },
      });
    });
  });

  it('disables the calendar rule while auto-stop is off', async () => {
    const settings = defaultSettings();
    const autoStop = { ...settings.recording.autoStop, enabled: false };
    bindings.settings.get.mockResolvedValue({
      status: 'ok',
      data: { ...settings, recording: { ...settings.recording, autoStop } },
    });
    renderSection();
    await openMoreOptions();

    const calendar = await screen.findByRole('switch', { name: copy.calendarTitle });

    expect(calendar.hasAttribute('disabled')).toBe(true);
  });
});
