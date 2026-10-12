import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { defaultSettings } from '../settings-fixture';
import { CaptureModeSettings } from './capture-mode-settings';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

function renderModes() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <CaptureModeSettings />
    </QueryClientProvider>,
  );
}

describe('Settings → Transcription → While you record', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
  });

  test('live transcription is on by default and turning it off records audio only', async () => {
    const off = defaultSettings();
    off.transcription.liveTranscription = false;
    settings.patch.mockResolvedValue({ status: 'ok', data: off });
    renderModes();

    const live = await screen.findByRole('switch', { name: 'Live transcription' });
    expect(live.getAttribute('aria-checked')).toBe('true');
    fireEvent.click(live);

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ transcription: { liveTranscription: false } });
    });
    expect(await screen.findByText(/only audio is recorded/u)).toBeDefined();
    const repass = screen.getByRole('switch', {
      name: 'Re-transcribe after the meeting for better accuracy',
    });
    expect(repass).toHaveProperty('disabled', true);
  });

  test('the re-pass is off by default and saved when turned on', async () => {
    const on = defaultSettings();
    on.transcription.highAccuracyRepass = true;
    settings.patch.mockResolvedValue({ status: 'ok', data: on });
    renderModes();

    fireEvent.click(
      await screen.findByRole('switch', {
        name: 'Re-transcribe after the meeting for better accuracy',
      }),
    );

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ transcription: { highAccuracyRepass: true } });
    });
  });
});
