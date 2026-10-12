import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { defaultSettings } from '../settings-fixture';
import { RedactionSettings } from './redaction-settings';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

function renderRedaction() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RedactionSettings />
    </QueryClientProvider>,
  );
}

describe('Settings → Network → Sensitive information', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
  });

  test('redaction is off by default and saved when turned on', async () => {
    const on = defaultSettings();
    on.privacy.redactSensitive = true;
    settings.patch.mockResolvedValue({ status: 'ok', data: on });
    renderRedaction();

    const toggle = await screen.findByRole('switch', { name: 'Hide sensitive information' });
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText(/include everything that was said/u)).toBeDefined();
    fireEvent.click(toggle);

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ privacy: { redactSensitive: true } });
    });
    expect(await screen.findByText(/become \[redacted\]/u)).toBeDefined();
  });
});
