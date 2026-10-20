import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { defaultSettings } from '../../settings/settings-fixture';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { diagnosticsCopy } from '../copy';
import { CallWindowCapture } from './call-window-capture';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

function renderCapture() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <CallWindowCapture />
    </QueryClientProvider>,
  );
}

describe('Settings → Developer → Diagnostics', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
  });

  test('saving the call window is off by default and saved when turned on', async () => {
    const on = defaultSettings();
    on.developer.captureCallWindows = true;
    settings.patch.mockResolvedValue({ status: 'ok', data: on });
    renderCapture();

    const toggle = await screen.findByRole('switch', { name: diagnosticsCopy.captureLabel });
    await waitFor(() => {
      expect(toggle.hasAttribute('disabled')).toBe(false);
    });
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText(diagnosticsCopy.captureDescription)).toBeDefined();
    fireEvent.click(toggle);

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ developer: { captureCallWindows: true } });
    });
    await waitFor(() => {
      expect(toggle.getAttribute('aria-checked')).toBe('true');
    });
  });

  test('a saved setting shows the switch on', async () => {
    const on = defaultSettings();
    on.developer.captureCallWindows = true;
    settings.get.mockResolvedValue({ status: 'ok', data: on });
    renderCapture();

    const toggle = await screen.findByRole('switch', { name: diagnosticsCopy.captureLabel });
    await waitFor(() => {
      expect(toggle.getAttribute('aria-checked')).toBe('true');
    });
  });
});
