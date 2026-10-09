import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/transcription';
import { createQueryClient } from '@/lib/ipc';

import { copy } from '../copy';
import type { ModelFileId, ModelFileView } from '../types';
import { ParakeetUnifiedManager } from './parakeet-unified-manager';

// The generated bindings, mocked with their real types.
const bindings = vi.hoisted(() => {
  const progress: { handler?: EventCallback<Generated.ModelDownloadProgress> } = {};
  const commands = {
    listModels: vi.fn<typeof Generated.commands.listModels>(),
    downloadModel: vi.fn<typeof Generated.commands.downloadModel>(),
    cancelDownload: vi.fn<typeof Generated.commands.cancelDownload>(),
    deleteModel: vi.fn<typeof Generated.commands.deleteModel>(),
  } satisfies typeof Generated.commands;
  const unlisten = () => Promise.resolve(vi.fn());
  const events = {
    modelChanged: { listen: vi.fn(unlisten) },
    modelDownloadProgress: {
      listen: vi.fn((handler: EventCallback<Generated.ModelDownloadProgress>) => {
        progress.handler = handler;
        return Promise.resolve(vi.fn());
      }),
    },
  };
  return { commands, events, progress };
});
vi.mock('@/lib/ipc/generated/transcription', () => ({
  commands: bindings.commands,
  events: bindings.events,
}));

function file(quant: Generated.Quant, ready: boolean, recommended = false): ModelFileView {
  return {
    id: `parakeet-unified-en-0.6b-${quant}` as ModelFileId,
    name: 'Parakeet Unified EN 0.6B',
    quant,
    sizeBytes: 731_000_000,
    state: ready ? { kind: 'ready' } : { kind: 'missing' },
    recommended,
  };
}

function renderManager(selected: string | undefined, onModelSelect = vi.fn()) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ParakeetUnifiedManager selectedModel={selected} onModelSelect={onModelSelect} />
    </QueryClientProvider>,
  );
  return onModelSelect;
}

async function row(quant: string): Promise<HTMLElement> {
  const label = await screen.findByText(quant, { exact: false });
  const item = label.closest('li');
  if (item === null) throw new Error(`no row for ${quant}`);
  return item;
}

describe('ParakeetUnifiedManager', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bindings.commands.listModels.mockResolvedValue({
      status: 'ok',
      data: [file('Q5_K_M', true), file('Q8_0', false, true)],
    });
  });

  it('marks the recommended quant and the file in use', async () => {
    renderManager('parakeet-unified-en-0.6b-Q5_K_M');

    const q5 = await row('Q5_K_M');
    const q8 = await row('Q8_0');

    expect(within(q5).getByRole('button', { name: copy.inUse })).toBeDefined();
    expect(within(q8).getByText(copy.recommended)).toBeDefined();
    expect(within(q8).getByText(/Not downloaded/)).toBeDefined();
  });

  it('downloads a file, shows its progress, and selects it when it is ready', async () => {
    let finish: (value: Awaited<ReturnType<typeof Generated.commands.downloadModel>>) => void =
      vi.fn();
    bindings.commands.downloadModel.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const onModelSelect = renderManager(undefined);

    fireEvent.click(within(await row('Q8_0')).getByRole('button', { name: copy.download }));
    act(() => {
      const progress = {
        id: file('Q8_0', false).id,
        downloadedBytes: 365_500_000,
        totalBytes: 731_000_000,
      };
      bindings.progress.handler?.({ event: 'progress', id: 1, payload: { progress } });
    });

    expect(await screen.findByText(/Downloading… 50%/)).toBeDefined();
    await act(async () => {
      finish({ status: 'ok', data: null });
      await Promise.resolve();
    });
    expect(bindings.commands.downloadModel).toHaveBeenCalledWith('parakeet-unified-en-0.6b-Q8_0');
    expect(onModelSelect).toHaveBeenCalledWith('parakeet-unified-en-0.6b-Q8_0');
  });

  it('shows why a download failed', async () => {
    bindings.commands.downloadModel.mockResolvedValue({
      status: 'error',
      error: {
        code: 'network',
        message: 'the downloaded file is corrupt and was deleted; try again',
      },
    });
    renderManager(undefined);

    fireEvent.click(within(await row('Q8_0')).getByRole('button', { name: copy.download }));

    expect(await screen.findByText(/corrupt and was deleted/)).toBeDefined();
  });
});
