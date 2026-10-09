import { type QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { EventCallback } from '@tauri-apps/api/event';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Secrets from '@/lib/ipc/generated/secrets';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient, IpcError } from '@/lib/ipc';

import {
  settingsKeys,
  usePatchSettings,
  useSaveSecret,
  useSecretStatus,
  useSettings,
  useSettingsChanges,
} from './queries';
import { defaultSettings } from './settings-fixture';

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => {
  const changed: { handler?: EventCallback<Generated.SettingsChanged> } = {};
  const settings = {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands;
  const secrets = {
    set: vi.fn<typeof Secrets.commands.set>(),
    status: vi.fn<typeof Secrets.commands.status>(),
    delete: vi.fn<typeof Secrets.commands.delete>(),
  } satisfies typeof Secrets.commands;
  const listen = vi.fn((handler: EventCallback<Generated.SettingsChanged>) => {
    changed.handler = handler;
    return Promise.resolve(vi.fn());
  });
  return { settings, secrets, changed, events: { settingsChanged: { listen } } };
});
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: bindings.settings,
  events: bindings.events,
}));
vi.mock('@/lib/ipc/generated/secrets', () => ({ commands: bindings.secrets }));

function renderWithClient<T>(hook: () => T): { result: { current: T }; client: QueryClient } {
  const client = createQueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const { result } = renderHook(hook, { wrapper });
  return { result, client };
}

describe('settings queries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bindings.settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
  });

  it('loads every setting through the settings plugin', async () => {
    const { result } = renderWithClient(useSettings);

    await waitFor(() => {
      expect(result.current.data).toEqual(defaultSettings());
    });
  });

  it('caches the saved settings a patch returns', async () => {
    const dark = { ...defaultSettings(), ui: { ...defaultSettings().ui, theme: 'dark' as const } };
    bindings.settings.patch.mockResolvedValue({ status: 'ok', data: dark });
    const { result, client } = renderWithClient(usePatchSettings);

    await act(() => result.current.mutateAsync({ ui: { theme: 'dark' } }));

    expect(bindings.settings.patch).toHaveBeenCalledWith({ ui: { theme: 'dark' } });
    expect(client.getQueryData(settingsKeys.document())).toEqual(dark);
  });

  it('surfaces a rejected patch as an invalid IpcError', async () => {
    bindings.settings.patch.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'there is no setting named ui.colour' },
    });
    const { result } = renderWithClient(usePatchSettings);

    const failed = result.current.mutateAsync({ ui: { theme: 'light' } });

    await expect(failed).rejects.toBeInstanceOf(IpcError);
  });

  it('writes every SettingsChanged event into the cache', async () => {
    const { client } = renderWithClient(useSettingsChanges);
    const changed = { ...defaultSettings(), ai: { ...defaultSettings().ai, autoSummary: true } };
    await waitFor(() => {
      expect(bindings.changed.handler).toBeDefined();
    });

    act(() => {
      bindings.changed.handler?.({
        event: 'plugin:settings:settings-changed',
        id: 1,
        payload: { settings: changed },
      });
    });

    expect(client.getQueryData(settingsKeys.document())).toEqual(changed);
  });
});

describe('secret queries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reports a key macOS refuses to hand over as locked', async () => {
    bindings.secrets.status.mockResolvedValue({ status: 'ok', data: 'locked' });
    const { result } = renderWithClient(() => useSecretStatus('openai_api_key'));

    await waitFor(() => {
      expect(result.current.data).toBe('locked');
    });
  });

  it('asks nothing for a provider without a key', () => {
    renderWithClient(() => useSecretStatus(null));

    expect(bindings.secrets.status).not.toHaveBeenCalled();
  });

  it('marks a saved key as stored without reading it back', async () => {
    bindings.secrets.set.mockResolvedValue({ status: 'ok', data: null });
    const { result, client } = renderWithClient(useSaveSecret);

    await act(() => result.current.mutateAsync({ name: 'groq_api_key', value: 'gsk-1' }));

    expect(bindings.secrets.set).toHaveBeenCalledWith('groq_api_key', 'gsk-1');
    expect(client.getQueryData(settingsKeys.secret('groq_api_key'))).toBe('stored');
  });
});
