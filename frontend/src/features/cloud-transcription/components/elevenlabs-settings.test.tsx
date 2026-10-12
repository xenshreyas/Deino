import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Cloud from '@/lib/ipc/generated/cloud-transcription';
import type * as Generated from '@/lib/ipc/generated/settings';
import { DEFAULT_SETTINGS } from '@/lib/ipc/generated/settings-defaults';
import { createQueryClient } from '@/lib/ipc';

import { ElevenLabsSettings } from './elevenlabs-settings';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

const cloud = vi.hoisted(() => ({
  keyStatus: vi.fn<typeof Cloud.commands.keyStatus>(),
  saveKey: vi.fn<typeof Cloud.commands.saveKey>(),
  checkKey: vi.fn<typeof Cloud.commands.checkKey>(),
  forgetKey: vi.fn<typeof Cloud.commands.forgetKey>(),
}));
vi.mock('@/lib/ipc/generated/cloud-transcription', () => ({ commands: cloud }));

/** A fresh copy of the settings Rust starts with. */
function defaultSettings(): Generated.Settings {
  return structuredClone(DEFAULT_SETTINGS);
}

function renderCard() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ElevenLabsSettings />
    </QueryClientProvider>,
  );
}

const SWITCH = 'Transcribe meetings with ElevenLabs after they end';

describe('Settings → Transcription → ElevenLabs (optional)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
    cloud.keyStatus.mockResolvedValue({ status: 'ok', data: 'missing' });
  });

  test('says audio leaves the Mac and keeps the switch off until a key is saved', async () => {
    cloud.saveKey.mockResolvedValue({ status: 'ok', data: null });
    renderCard();

    expect(await screen.findByText(/Live transcription stays on this Mac/u)).toBeDefined();
    const toggle = await screen.findByRole('switch', { name: SWITCH });
    expect(toggle).toHaveProperty('disabled', true);
    fireEvent.change(screen.getByLabelText('API key'), { target: { value: ' xi-key ' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(cloud.saveKey).toHaveBeenCalledWith('xi-key');
    });
    expect(await screen.findByRole('button', { name: 'Check' })).toBeDefined();
    expect(screen.getByRole('switch', { name: SWITCH })).toHaveProperty('disabled', false);
  });

  test('with a key saved, the switch turns ElevenLabs on and Check reports the answer', async () => {
    const on = defaultSettings();
    on.transcription.cloudProvider = 'elevenlabs';
    cloud.keyStatus.mockResolvedValue({ status: 'ok', data: 'stored' });
    cloud.checkKey.mockResolvedValue({
      status: 'error',
      error: {
        code: 'permission_denied',
        message: 'ElevenLabs refused the API key. Check it and save it again.',
      },
    });
    settings.patch.mockResolvedValue({ status: 'ok', data: on });
    renderCard();

    fireEvent.click(await screen.findByRole('switch', { name: SWITCH }));
    fireEvent.click(screen.getByRole('button', { name: 'Check' }));

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({
        transcription: { cloudProvider: 'elevenlabs' },
      });
    });
    expect(await screen.findByText(/ElevenLabs refused the API key/u)).toBeDefined();
  });

  test('forgetting the key brings the key box back', async () => {
    cloud.keyStatus.mockResolvedValue({ status: 'ok', data: 'stored' });
    cloud.forgetKey.mockResolvedValue({ status: 'ok', data: null });
    renderCard();

    fireEvent.click(await screen.findByRole('button', { name: 'Forget' }));

    expect(await screen.findByLabelText('API key')).toBeDefined();
    expect(cloud.forgetKey).toHaveBeenCalledOnce();
  });
});
