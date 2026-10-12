import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/api';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { apiCopy } from '../copy';
import type { ApiKey, ApiKeyId, ApiStatus } from '../types';
import { LocalApiSettings } from './local-api-settings';
import { parsePort } from './server-section';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      serverStatus: vi.fn<typeof Generated.commands.serverStatus>(),
      configureServer: vi.fn<typeof Generated.commands.configureServer>(),
      listKeys: vi.fn<typeof Generated.commands.listKeys>(),
      createKey: vi.fn<typeof Generated.commands.createKey>(),
      revokeKey: vi.fn<typeof Generated.commands.revokeKey>(),
      deleteKey: vi.fn<typeof Generated.commands.deleteKey>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/api', () => ({ commands }));

const OFF: ApiStatus = { enabled: false, port: 47810, baseUrl: null, error: null };
const KEY: ApiKey = {
  id: '0199d6f0-0000-7000-8000-000000000001' as ApiKeyId,
  name: 'Raycast',
  scope: 'read',
  createdAt: '2026-10-01T10:00:00Z',
  lastUsedAt: null,
  revokedAt: null,
};
const TOKEN = 'deino_0199d6f0000070008000000000000002_abc';

function renderSettings() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <LocalApiSettings />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commands.serverStatus.mockResolvedValue({ status: 'ok', data: OFF });
  commands.listKeys.mockResolvedValue({ status: 'ok', data: [KEY] });
});

describe('LocalApiSettings', () => {
  it('turns the server on at its port and shows where it listens', async () => {
    const on = { ...OFF, enabled: true, baseUrl: 'http://127.0.0.1:47810/v1' };
    commands.configureServer.mockResolvedValue({ status: 'ok', data: on });
    renderSettings();

    fireEvent.click(await screen.findByRole('switch', { name: apiCopy.enable }));

    expect(await screen.findByText(apiCopy.listening(on.baseUrl))).toBeDefined();
    expect(commands.configureServer).toHaveBeenCalledWith({ enabled: true, port: 47810 });
  });

  it('shows a new key once, then only its name', async () => {
    const created = { ...KEY, id: 'k2' as ApiKeyId, name: 'Cron', scope: 'read_write' as const };
    commands.createKey.mockResolvedValue({ status: 'ok', data: { key: created, token: TOKEN } });
    renderSettings();

    fireEvent.change(await screen.findByLabelText(apiCopy.name), { target: { value: 'Cron' } });
    fireEvent.click(screen.getByRole('radio', { name: apiCopy.scopes.read_write }));
    fireEvent.click(screen.getByRole('button', { name: apiCopy.create }));

    expect(await screen.findByText(TOKEN)).toBeDefined();
    expect(commands.createKey).toHaveBeenCalledWith({ name: 'Cron', scope: 'read_write' });
    fireEvent.click(screen.getByRole('button', { name: apiCopy.done }));
    await waitFor(() => {
      expect(screen.queryByText(TOKEN)).toBeNull();
    });
  });

  it('lists keys with their use and revokes one', async () => {
    commands.revokeKey.mockResolvedValue({
      status: 'ok',
      data: { ...KEY, revokedAt: '2026-10-12T09:00:00Z' },
    });
    renderSettings();

    expect(await screen.findByText('Raycast')).toBeDefined();
    expect(screen.getByText(/Read only · Created Oct 1, 2026, never used/)).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: apiCopy.revoke }));

    await waitFor(() => {
      expect(commands.revokeKey).toHaveBeenCalledWith(KEY.id);
    });
  });
});

describe('parsePort', () => {
  it('accepts unprivileged ports only', () => {
    expect(parsePort(' 8080 ')).toBe(8080);
    expect(parsePort('80')).toBeNull();
    expect(parsePort('70000')).toBeNull();
    expect(parsePort('12.5')).toBeNull();
    expect(parsePort('')).toBeNull();
  });
});
