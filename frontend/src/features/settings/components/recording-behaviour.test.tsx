import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { recordingSettingsCopy as copy } from '../copy';
import { defaultSettings } from '../settings-fixture';
import { RecordingBehaviour } from './recording-behaviour';

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => ({
  settings: {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands,
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: bindings.settings,
  events: { settingsChanged: { listen: bindings.listen } },
}));

function renderSection() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RecordingBehaviour />
    </QueryClientProvider>,
  );
}

describe('Settings → Recording, while recording', () => {
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

  it('keeps the calendar rule off by default and turns it on alone', async () => {
    renderSection();
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

    const calendar = await screen.findByRole('switch', { name: copy.calendarTitle });

    expect(calendar.hasAttribute('disabled')).toBe(true);
  });
});
