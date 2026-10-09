import { QueryClientProvider } from '@tanstack/react-query';
import { act, render, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { SettingsSync } from './components/settings-sync';
import { defaultSettings } from './settings-fixture';
import { useSaveTheme, useThemePreference } from './theme';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: settings,
  events: { settingsChanged: { listen: vi.fn(() => Promise.resolve(vi.fn())) } },
}));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ setTheme: vi.fn() }) }));
vi.mock('./legacy-local-storage', () => ({ migrateLegacyLocalStorage: () => Promise.resolve() }));

/** Saved settings with `theme`. */
function savedWith(theme: Generated.Theme): Generated.Settings {
  const base = defaultSettings();
  return { ...base, ui: { ...base.ui, theme } };
}

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe('theme setting', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete document.documentElement.dataset.theme;
  });

  it('applies the saved theme once the settings load', async () => {
    settings.get.mockResolvedValue({ status: 'ok', data: savedWith('dark') });

    render(<SettingsSync />, { wrapper });

    await waitFor(() => {
      expect(document.documentElement.dataset.theme).toBe('dark');
    });
  });

  it('a new choice shows at once and is saved as ui.theme', async () => {
    settings.get.mockResolvedValue({ status: 'ok', data: savedWith('system') });
    settings.patch.mockResolvedValue({ status: 'ok', data: savedWith('light') });
    const { result } = renderHook(() => ({ save: useSaveTheme(), current: useThemePreference() }), {
      wrapper,
    });
    await waitFor(() => {
      expect(result.current.current).toBe('system');
    });

    act(() => {
      result.current.save('light');
    });

    expect(document.documentElement.dataset.theme).toBe('light');
    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ ui: { theme: 'light' } });
    });
    await waitFor(() => {
      expect(result.current.current).toBe('light');
    });
  });
});
