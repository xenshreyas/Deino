import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { settingsCopy } from '../copy';
import { defaultSettings } from '../settings-fixture';
import type { Settings, TranscriptionEngine } from '../types';
import { EngineSettings, isEngine } from './engine-settings';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

/** A model manager stand-in: names its engine and selection, and can pick. */
function fakeManager(engine: string) {
  return ({
    selectedModel,
    onModelSelect,
  }: {
    selectedModel?: string;
    onModelSelect?: (model: string) => void;
  }) => (
    <button type="button" onClick={() => onModelSelect?.(`${engine}-picked`)}>
      {`${engine} manager: ${selectedModel ?? 'none'}`}
    </button>
  );
}
vi.mock('@/features/transcription', () => ({ ParakeetUnifiedManager: fakeManager('unified') }));
vi.mock('@/components/ParakeetModelManager', () => ({ ParakeetModelManager: fakeManager('tdt') }));
vi.mock('@/components/WhisperModelManager', () => ({ ModelManager: fakeManager('whisper') }));

function saved(engine: TranscriptionEngine, model: string): Settings {
  const value = defaultSettings();
  value.transcription.engine = engine;
  value.transcription.model = model;
  return value;
}

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <EngineSettings />
    </QueryClientProvider>,
  );
}

const names = settingsCopy.engine.names;

describe('Settings → Transcription → engine', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('a stored parakeet engine shows as Parakeet TDT, not Unified', async () => {
    settings.get.mockResolvedValue({
      status: 'ok',
      data: saved('parakeet', 'parakeet-tdt-0.6b-v3-int8'),
    });
    renderPage();

    const select = await screen.findByRole('combobox', { name: settingsCopy.engine.label });
    expect(select.textContent).toBe(names.parakeet);
    expect(screen.getByText('tdt manager: parakeet-tdt-0.6b-v3-int8')).toBeDefined();
    expect(screen.queryByText(/unified manager/u)).toBeNull();
    expect(settings.patch).not.toHaveBeenCalled();
  });

  test('a stored Parakeet Unified file shows as Unified with that file selected', async () => {
    settings.get.mockResolvedValue({
      status: 'ok',
      data: saved('parakeet_unified', 'parakeet-unified-en-0.6b-Q8_0'),
    });
    renderPage();

    const select = await screen.findByRole('combobox', { name: settingsCopy.engine.label });
    expect(select.textContent).toBe(names.parakeet_unified);
    expect(screen.getByText('unified manager: parakeet-unified-en-0.6b-Q8_0')).toBeDefined();
  });

  test('no engine is shown until the saved settings arrive', () => {
    settings.get.mockReturnValue(new Promise(() => undefined));
    renderPage();

    expect(screen.getByText(settingsCopy.engine.loading)).toBeDefined();
    expect(screen.queryByRole('combobox')).toBeNull();
  });

  test('picking a model saves the engine and the model together', async () => {
    const before = saved('whisper', '');
    settings.get.mockResolvedValue({ status: 'ok', data: before });
    settings.patch.mockResolvedValue({ status: 'ok', data: saved('whisper', 'whisper-picked') });
    renderPage();

    fireEvent.click(await screen.findByRole('button', { name: /whisper manager/u }));

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({
        transcription: { engine: 'whisper', model: 'whisper-picked' },
      });
    });
  });

  test('only engine names pass the select guard', () => {
    expect(isEngine('parakeet')).toBe(true);
    expect(isEngine('parakeetUnified')).toBe(false);
    expect(isEngine('toString')).toBe(false);
  });
});
