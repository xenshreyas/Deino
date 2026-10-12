import { QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/importers';
import { createQueryClient } from '@/lib/ipc';

import { importCopy, phaseCopy, sourceCopy } from '../copy';
import type { ImportProgress, ImportSourceStatus } from '../types';
import { ImportSettings } from './import-settings';

// The generated bindings, mocked with their real types: a Rust signature
// change makes this file fail to compile instead of silently passing.
const bindings = vi.hoisted(() => {
  const progressChanged: { handler?: EventCallback<Generated.ImportProgressChanged> } = {};
  const commands = {
    sources: vi.fn<typeof Generated.commands.sources>(),
    saveKey: vi.fn<typeof Generated.commands.saveKey>(),
    forgetKey: vi.fn<typeof Generated.commands.forgetKey>(),
    pickFolder: vi.fn<typeof Generated.commands.pickFolder>(),
    preview: vi.fn<typeof Generated.commands.preview>(),
    start: vi.fn<typeof Generated.commands.start>(),
    cancel: vi.fn<typeof Generated.commands.cancel>(),
    status: vi.fn<typeof Generated.commands.status>(),
  } satisfies typeof Generated.commands;
  const listen = vi.fn((handler: EventCallback<Generated.ImportProgressChanged>) => {
    progressChanged.handler = handler;
    return Promise.resolve(vi.fn());
  });
  return { commands, events: { importProgressChanged: { listen } }, progressChanged };
});
vi.mock('@/lib/ipc/generated/importers', () => ({
  commands: bindings.commands,
  events: bindings.events,
}));
const { commands } = bindings;

const IDLE: ImportProgress = {
  source: null,
  phase: 'idle',
  total: 0,
  imported: 0,
  skipped: 0,
  failed: 0,
  error: null,
};
const SOURCES: ImportSourceStatus[] = [
  { source: 'granola', key: 'stored' },
  { source: 'circleback', key: 'missing' },
  { source: 'fireflies', key: 'missing' },
  { source: 'fathom', key: 'missing' },
  { source: 'folder', key: 'stored' },
];

function renderSettings() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ImportSettings />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commands.sources.mockResolvedValue({ status: 'ok', data: SOURCES });
  commands.status.mockResolvedValue({ status: 'ok', data: IDLE });
});

describe('ImportSettings', () => {
  it('counts first, then imports and follows the progress events', async () => {
    const running: ImportProgress = {
      ...IDLE,
      source: 'granola',
      phase: 'importing',
      total: 36,
      skipped: 12,
    };
    commands.preview.mockResolvedValue({
      status: 'ok',
      data: { source: 'granola', total: 48, alreadyImported: 12, new: 36 },
    });
    commands.start.mockResolvedValue({ status: 'ok', data: running });
    renderSettings();

    await screen.findByText(importCopy.keySaved);
    const importButtons = screen.getAllByRole('button', { name: importCopy.import });
    expect(importButtons.every((button) => button.hasAttribute('disabled'))).toBe(true);
    fireEvent.click(screen.getAllByRole('button', { name: importCopy.preview })[0] as HTMLElement);
    await vi.waitFor(() => {
      expect(commands.preview).toHaveBeenCalledWith({ source: 'granola', folder: null });
    });
    expect(await screen.findByText(importCopy.previewResult(36, 12))).toBeDefined();
    fireEvent.click(screen.getAllByRole('button', { name: importCopy.import })[0] as HTMLElement);

    expect(await screen.findByRole('progressbar')).toBeDefined();
    expect(commands.start).toHaveBeenCalledWith({ source: 'granola', folder: null });
    act(() => {
      bindings.progressChanged.handler?.({
        event: 'plugin:importers:import-progress-changed',
        id: 1,
        payload: { progress: { ...running, phase: 'finished', imported: 36 } },
      });
    });
    expect(
      await screen.findByText(`${sourceCopy.granola.name} · ${phaseCopy.finished}`),
    ).toBeDefined();
    expect(screen.getByText(importCopy.summary(36, 12, 0))).toBeDefined();
  });

  it('saves a pasted key and never shows it again', async () => {
    commands.saveKey.mockResolvedValue({ status: 'ok', data: null });
    renderSettings();

    const field = await screen.findByLabelText(importCopy.keyLabel(sourceCopy.circleback.name));
    fireEvent.change(field, { target: { value: ' cb_secret ' } });
    fireEvent.click(screen.getAllByRole('button', { name: importCopy.saveKey })[0] as HTMLElement);

    await vi.waitFor(() => {
      expect(commands.saveKey).toHaveBeenCalledWith('circleback', 'cb_secret');
    });
    expect(commands.sources).toHaveBeenCalledTimes(2);
  });

  it('stops a running import', async () => {
    const running: ImportProgress = {
      ...IDLE,
      source: 'fathom',
      phase: 'importing',
      total: 10,
      imported: 4,
    };
    commands.status.mockResolvedValue({ status: 'ok', data: running });
    commands.cancel.mockResolvedValue({ status: 'ok', data: true });
    renderSettings();

    expect(await screen.findByText(importCopy.progress(4, 10))).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: importCopy.cancel }));

    await vi.waitFor(() => {
      expect(commands.cancel).toHaveBeenCalled();
    });
  });

  it('shows why an import failed', async () => {
    const failed: ImportProgress = {
      ...IDLE,
      source: 'circleback',
      phase: 'failed',
      error: 'circleback did not accept the API key; check it and save it again',
    };
    commands.status.mockResolvedValue({ status: 'ok', data: failed });
    renderSettings();

    expect(await screen.findByRole('alert')).toBeDefined();
    expect(screen.getByText(failed.error ?? '')).toBeDefined();
  });
});
