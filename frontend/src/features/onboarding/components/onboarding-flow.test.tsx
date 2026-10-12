import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Calendar from '@/lib/ipc/generated/calendar';
import type * as Onboarding from '@/lib/ipc/generated/onboarding';
import type * as Profile from '@/lib/ipc/generated/profile';
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
    importSample: vi.fn<typeof Onboarding.commands.importSample>(),
  } satisfies typeof Onboarding.commands,
  profile: {
    update: vi.fn<typeof Profile.commands.update>(),
    context: vi.fn<typeof Profile.commands.context>(),
  } satisfies typeof Profile.commands,
  calendarStatus: vi.fn<typeof Calendar.commands.status>(),
  navigate: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: bindings.settings, events: {} }));
vi.mock('@/lib/ipc/generated/onboarding', () => ({ commands: bindings.onboarding }));
vi.mock('@/lib/ipc/generated/profile', () => ({ commands: bindings.profile }));
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => bindings.navigate }));
vi.mock('@/lib/ipc/generated/calendar', () => ({
  commands: { status: bindings.calendarStatus },
  events: {},
}));

/** The saved profile: a name only. */
const PROFILE: Profile.ProfileSettings = {
  name: 'Shreyas',
  emails: [],
  role: '',
  company: '',
  focusAreas: [],
  writingStyle: '',
  context: '',
};

/** Settings whose onboarding section is `onboarding`; the rest is never read here. */
function settingsWith(onboarding: OnboardingSettings): Generated.Settings {
  // The flow reads only `onboarding`, `ai` and `profile`; a partial document is enough.
  return {
    onboarding,
    ai: { provider: 'claude', providerModels: {} },
    profile: PROFILE,
  } as Generated.Settings;
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

  it('asks only for what recording needs: the microphone and system audio (UX-21, UX-22)', async () => {
    renderFlow({ ...FRESH, step: 'permissions' });

    expect(await screen.findByText('Not checked yet')).toBeTruthy();
    const list = screen.getByRole('list', { name: 'Permissions' });
    expect(within(list).getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: 'Allow' })).toHaveLength(1);
    for (const name of ['Calendar', 'Notifications', 'Accessibility', 'Screen Recording']) {
      expect(within(list).queryByText(name)).toBeNull();
    }
    expect(screen.queryByRole('button', { name: 'Open System Settings' })).toBeNull();
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

  it('completes when the last step, Profile, is skipped (UX-24)', async () => {
    renderFlow({ ...FRESH, step: 'profile' });

    expect(await screen.findByRole('list', { name: 'Step 5 of 5' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Skip' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        onboarding: { completed: true, step: 'profile', outcomes: { profile: 'skipped' } },
      });
    });
  });

  it('saves profile edits before moving on', async () => {
    bindings.profile.update.mockImplementation((input) =>
      Promise.resolve({ status: 'ok', data: { ...PROFILE, ...input } }),
    );
    renderFlow({ ...FRESH, step: 'profile' });

    fireEvent.change(await screen.findByLabelText('Company'), { target: { value: 'Deino' } });
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        onboarding: { completed: true, step: 'profile', outcomes: { profile: 'done' } },
      });
    });
    expect(bindings.profile.update).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'Shreyas', company: 'Deino' }),
    );
  });

  it('moves on without saving an untouched profile', async () => {
    renderFlow({ ...FRESH, step: 'profile' });

    await screen.findByLabelText('Company');
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalled();
    });
    expect(bindings.profile.update).not.toHaveBeenCalled();
  });

  it('stays on the profile step when the save fails', async () => {
    bindings.profile.update.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'disk full' },
    });
    renderFlow({ ...FRESH, step: 'profile' });

    fireEvent.change(await screen.findByLabelText('Role'), { target: { value: 'CTO' } });
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));

    expect((await screen.findByRole('alert')).textContent).toContain('disk full');
    expect(bindings.settings.patch).not.toHaveBeenCalled();
  });

  it('waits for a move to be saved before taking the next one', async () => {
    bindings.settings.patch.mockReturnValue(new Promise(() => undefined));
    renderFlow();

    fireEvent.click(await screen.findByRole('button', { name: 'Get started' }));

    const next = await screen.findByRole('button', { name: 'Continue' });
    expect(next.hasAttribute('disabled')).toBe(true);
    fireEvent.click(next);
    expect(bindings.settings.patch).toHaveBeenCalledTimes(1);
  });
});
