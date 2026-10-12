import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Calendar from '@/lib/ipc/generated/calendar';
import type * as Onboarding from '@/lib/ipc/generated/onboarding';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { FRESH } from '../machine';
import type { OnboardingSettings } from '../types';
import { OnboardingFlow } from './onboarding-flow';

const bindings = vi.hoisted(() => ({
  settings: {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands,
  onboarding: {
    permissions: vi.fn<typeof Onboarding.commands.permissions>(),
    requestPermission: vi.fn<typeof Onboarding.commands.requestPermission>(),
    openPermissionSettings: vi.fn<typeof Onboarding.commands.openPermissionSettings>(),
  } satisfies typeof Onboarding.commands,
  calendarStatus: vi.fn<typeof Calendar.commands.status>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: bindings.settings, events: {} }));
vi.mock('@/lib/ipc/generated/onboarding', () => ({ commands: bindings.onboarding }));
vi.mock('@/lib/ipc/generated/calendar', () => ({
  commands: { status: bindings.calendarStatus },
  events: {},
}));

/** Settings whose onboarding section is `onboarding`; the rest is never read here. */
function settingsWith(onboarding: OnboardingSettings): Generated.Settings {
  // The flow reads only `onboarding` and `ai`; a partial document is enough.
  return { onboarding, ai: { provider: 'claude', providerModels: {} } } as Generated.Settings;
}

/** Saves patches the way Rust does: the onboarding section is replaced. */
function savePatches() {
  bindings.settings.patch.mockImplementation((patch) => {
    const onboarding = (patch as { onboarding: OnboardingSettings }).onboarding;
    return Promise.resolve({ status: 'ok', data: settingsWith(onboarding) });
  });
}

function renderFlow(onboarding: OnboardingSettings = FRESH) {
  bindings.settings.get.mockResolvedValue({ status: 'ok', data: settingsWith(onboarding) });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <OnboardingFlow />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  savePatches();
  bindings.onboarding.permissions.mockResolvedValue({
    status: 'ok',
    data: [
      { kind: 'microphone', state: 'granted', optional: false },
      { kind: 'system_audio', state: 'unknown', optional: false },
      { kind: 'notifications', state: 'denied', optional: false },
      { kind: 'accessibility', state: 'not_determined', optional: true },
    ],
  });
  bindings.onboarding.requestPermission.mockResolvedValue({
    status: 'ok',
    data: { kind: 'system_audio', state: 'granted', optional: false },
  });
  bindings.onboarding.openPermissionSettings.mockResolvedValue({ status: 'ok', data: null });
  bindings.calendarStatus.mockResolvedValue({
    status: 'ok',
    data: {
      syncEnabled: false,
      access: 'not_determined',
      lastSyncedAt: null,
      lastError: null,
      connectors: [],
    },
  });
});

describe('OnboardingFlow', () => {
  it('starts a fresh install on the welcome step and saves each move', async () => {
    renderFlow();

    fireEvent.click(await screen.findByRole('button', { name: 'Get started' }));

    expect(await screen.findByRole('heading', { name: 'Permissions' })).toBeTruthy();
    expect(bindings.settings.patch).toHaveBeenCalledWith({
      onboarding: { completed: false, step: 'permissions', outcomes: { welcome: 'done' } },
    });
  });

  it('lets the whole setup be skipped from the welcome step', async () => {
    renderFlow();

    fireEvent.click(await screen.findByRole('button', { name: 'Skip setup' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        onboarding: { ...FRESH, completed: true },
      });
    });
  });

  it('resumes on the saved step', async () => {
    renderFlow({ completed: false, step: 'permissions', outcomes: { welcome: 'done' } });

    expect(await screen.findByRole('heading', { name: 'Permissions' })).toBeTruthy();
  });

  it('shows every permission with the button that moves it forward', async () => {
    renderFlow({ ...FRESH, step: 'permissions' });

    expect(await screen.findByText('Titles and attendees for your meetings.')).toBeTruthy();
    expect(screen.getByText('Not checked yet')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Allow' })).toHaveLength(3);
    expect(screen.getByRole('button', { name: 'Open System Settings' })).toBeTruthy();
  });

  it('asks for a permission and shows the answer', async () => {
    renderFlow({ ...FRESH, step: 'permissions' });

    const [systemAudio] = await screen.findAllByRole('button', { name: 'Allow' });
    if (systemAudio === undefined) throw new Error('no Allow button');
    fireEvent.click(systemAudio);

    await waitFor(() => {
      expect(screen.getAllByText('Allowed')).toHaveLength(2);
    });
    expect(bindings.onboarding.requestPermission).toHaveBeenCalledWith('system_audio');
  });

  it('goes back a step', async () => {
    renderFlow({ ...FRESH, step: 'permissions' });

    fireEvent.click(await screen.findByRole('button', { name: 'Back' }));

    expect(await screen.findByRole('heading', { name: 'Welcome to Deino' })).toBeTruthy();
  });

  it('completes when the last step is skipped', async () => {
    renderFlow({ ...FRESH, step: 'try_it' });

    fireEvent.click(await screen.findByRole('button', { name: 'Skip' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        onboarding: { completed: true, step: 'try_it', outcomes: { try_it: 'skipped' } },
      });
    });
  });
});
