import { QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { DEFAULT_SETTINGS } from '@/lib/ipc/generated/settings-defaults';
import type * as GeneratedSettings from '@/lib/ipc/generated/settings';
import type * as Generated from '@/lib/ipc/generated/transcription';
import { createQueryClient } from '@/lib/ipc';

import { useTranscriptionModel } from './use-local-models';

const transcription = vi.hoisted(() => ({
  commands: {
    listModels: vi.fn<typeof Generated.commands.listModels>(),
    downloadModel: vi.fn<typeof Generated.commands.downloadModel>(),
    cancelDownload: vi.fn<typeof Generated.commands.cancelDownload>(),
    deleteModel: vi.fn<typeof Generated.commands.deleteModel>(),
  } satisfies typeof Generated.commands,
  events: {
    modelChanged: { listen: vi.fn(() => Promise.resolve(vi.fn())) },
    modelDownloadProgress: { listen: vi.fn(() => Promise.resolve(vi.fn())) },
  },
}));
vi.mock('@/lib/ipc/generated/transcription', () => transcription);

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof GeneratedSettings.commands.get>(),
  patch: vi.fn<typeof GeneratedSettings.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: settings,
  events: { settingsChanged: { listen: vi.fn(() => Promise.resolve(vi.fn())) } },
}));

const Q8 = 'parakeet-unified-en-0.6b-Q8_0' as Generated.ModelFileId;

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe('onboarding → the Parakeet Unified download', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    transcription.commands.listModels.mockResolvedValue({
      status: 'ok',
      data: [
        {
          id: Q8,
          name: 'Parakeet Unified EN 0.6B',
          quant: 'Q8_0',
          sizeBytes: 731_000_000,
          state: { kind: 'missing' },
          recommended: true,
        },
      ],
    });
    transcription.commands.downloadModel.mockResolvedValue({ status: 'ok', data: null });
    settings.patch.mockResolvedValue({ status: 'ok', data: structuredClone(DEFAULT_SETTINGS) });
  });

  test('a finished download also selects Parakeet Unified with that file', async () => {
    const { result } = renderHook(useTranscriptionModel, { wrapper });
    await waitFor(() => {
      expect(result.current.sizeBytes).toBe(731_000_000);
    });

    act(() => {
      result.current.download();
    });

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({
        transcription: { engine: 'parakeet_unified', model: Q8 },
      });
    });
    expect(transcription.commands.downloadModel).toHaveBeenCalledWith(Q8);
  });

  test('a failed download leaves the engine alone', async () => {
    transcription.commands.downloadModel.mockResolvedValue({
      status: 'error',
      error: { code: 'network', message: 'offline' },
    });
    const { result } = renderHook(useTranscriptionModel, { wrapper });
    await waitFor(() => {
      expect(result.current.sizeBytes).toBe(731_000_000);
    });

    act(() => {
      result.current.download();
    });

    await waitFor(() => {
      expect(result.current.error).toBe('offline');
    });
    expect(settings.patch).not.toHaveBeenCalled();
  });
});
