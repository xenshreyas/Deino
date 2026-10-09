import { QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { useOfflineMode } from './offline-mode';
import { defaultSettings } from './settings-fixture';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe('offline mode setting', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
  });

  it('is off by default and saved as network.offlineMode', async () => {
    const offline = defaultSettings();
    offline.network.offlineMode = true;
    settings.patch.mockResolvedValue({ status: 'ok', data: offline });
    const { result } = renderHook(useOfflineMode, { wrapper });
    await waitFor(() => {
      expect(result.current.offlineMode).toBe(false);
    });

    act(() => {
      result.current.setOfflineMode(true);
    });

    await waitFor(() => {
      expect(result.current.offlineMode).toBe(true);
    });
    expect(settings.patch).toHaveBeenCalledWith({ network: { offlineMode: true } });
  });
});
