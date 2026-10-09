import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Detection from '@/lib/ipc/generated/detection';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { defaultSettings } from '../settings-fixture';
import { DetectionSettingsPage } from './detection-settings';

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => ({
  settings: {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands,
  detection: {
    current: vi.fn<typeof Detection.commands.current>(),
    takeNotes: vi.fn<typeof Detection.commands.takeNotes>(),
    dismiss: vi.fn<typeof Detection.commands.dismiss>(),
    ignoreApp: vi.fn<typeof Detection.commands.ignoreApp>(),
    settings: vi.fn<typeof Detection.commands.settings>(),
  } satisfies typeof Detection.commands,
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: bindings.settings,
  events: { settingsChanged: { listen: bindings.listen } },
}));
vi.mock('@/lib/ipc/generated/detection', () => ({
  commands: bindings.detection,
  events: {},
}));

/** Settings with Discord on the ignore list. */
function withDiscordIgnored(): Generated.Settings {
  const settings = defaultSettings();
  const discord = { bundleId: 'com.hnc.Discord' as Generated.BundleId, name: 'Discord' };
  return { ...settings, detection: { ...settings.detection, ignoredApps: [discord] } };
}

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <DetectionSettingsPage />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('Settings → Meeting detection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const settings = withDiscordIgnored();
    bindings.settings.get.mockResolvedValue({ status: 'ok', data: settings });
    bindings.settings.patch.mockResolvedValue({ status: 'ok', data: settings });
    bindings.detection.settings.mockResolvedValue({
      status: 'ok',
      data: { settings: settings.detection, supported: true, readsBrowserUrls: false },
    });
  });

  it('switches detection off with a merge patch', async () => {
    renderPage();

    fireEvent.click(await screen.findByRole('switch', { name: 'Detect meetings' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({ detection: { enabled: false } });
    });
  });

  it('keeps auto-start off by default and turns it on without changing the countdown', async () => {
    renderPage();
    const autoStart = await screen.findByRole('switch', { name: 'Start recording automatically' });

    expect(autoStart.getAttribute('aria-checked')).toBe('false');
    fireEvent.click(autoStart);

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        detection: { autoStart: { enabled: true, countdownSeconds: 10 } },
      });
    });
  });

  it('lists ignored apps and stops ignoring one', async () => {
    renderPage();

    expect(await screen.findByText('Discord')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Stop ignoring Discord' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({ detection: { ignoredApps: [] } });
    });
  });

  it('explains how to recognise browser calls faster without Accessibility', async () => {
    renderPage();

    expect(await screen.findByText(/Privacy & Security → Accessibility/)).toBeTruthy();
  });
});
