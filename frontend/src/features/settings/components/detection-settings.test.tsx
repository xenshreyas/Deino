import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Detection from '@/lib/ipc/generated/detection';
import type * as Generated from '@/lib/ipc/generated/settings';
import { defaultSettings } from '../settings-fixture';
import { renderInSettings } from '../test-render';
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
  return renderInSettings(DetectionSettingsPage);
}

/** Opens the section's "More options". */
async function openMoreOptions() {
  fireEvent.click(await screen.findByRole('button', { name: 'More options' }));
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

  it('shows only the detection switch until More options is opened', async () => {
    renderPage();

    expect(await screen.findAllByRole('switch')).toHaveLength(1);
    expect(screen.queryByText('Discord')).toBeNull();
  });

  it('keeps auto-start off by default and turns it on without changing the countdown', async () => {
    renderPage();
    await openMoreOptions();
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
    await openMoreOptions();

    expect(await screen.findByText('Discord')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Stop ignoring Discord' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({ detection: { ignoredApps: [] } });
    });
  });

  it('links to the Permissions card while Accessibility is missing', async () => {
    const router = renderPage();

    fireEvent.click(await screen.findByRole('link', { name: 'Needs Accessibility' }));

    await waitFor(() => {
      expect(router.state.location.search).toEqual({ section: 'permissions' });
    });
  });
});
