import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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

  it('saves profile edits before moving on', async () => {
    bindings.profile.update.mockImplementation((input) =>
      Promise.resolve({ status: 'ok', data: { ...PROFILE, ...input } }),
    );
    renderFlow({ ...FRESH, step: 'profile' });

    fireEvent.change(await screen.findByLabelText('Company'), { target: { value: 'Deino' } });
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        onboarding: { completed: false, step: 'try_it', outcomes: { profile: 'done' } },
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

  it('opens the sample meeting and finishes setup', async () => {
    bindings.onboarding.importSample.mockResolvedValue({
      status: 'ok',
      data: 'meeting-sample' as Onboarding.MeetingId,
    });
    renderFlow({ ...FRESH, step: 'try_it' });

    fireEvent.click(await screen.findByRole('button', { name: 'Open' }));

    await waitFor(() => {
      expect(bindings.navigate).toHaveBeenCalledWith({
        to: '/meetings/$meetingId',
        params: { meetingId: 'meeting-sample' },
      });
    });
    expect(bindings.settings.patch).toHaveBeenCalledWith({
      onboarding: { completed: true, step: 'try_it', outcomes: { try_it: 'done' } },
    });
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
