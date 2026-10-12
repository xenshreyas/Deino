import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Calendar from '@/lib/ipc/generated/calendar';
import type * as Onboarding from '@/lib/ipc/generated/onboarding';
import { createQueryClient } from '@/lib/ipc';

import { PermissionsSettings } from './permissions-settings';

const bindings = vi.hoisted(() => ({
  onboarding: {
    permissions: vi.fn<typeof Onboarding.commands.permissions>(),
    requestPermission: vi.fn<typeof Onboarding.commands.requestPermission>(),
    openPermissionSettings: vi.fn<typeof Onboarding.commands.openPermissionSettings>(),
    importSample: vi.fn<typeof Onboarding.commands.importSample>(),
  } satisfies typeof Onboarding.commands,
  calendarStatus: vi.fn<typeof Calendar.commands.status>(),
}));
vi.mock('@/lib/ipc/generated/onboarding', () => ({ commands: bindings.onboarding }));
vi.mock('@/lib/ipc/generated/calendar', () => ({
  commands: { status: bindings.calendarStatus },
  events: {},
}));

/** Every permission once, with Accessibility refused and Screen Recording unasked. */
const STATUSES: Onboarding.PermissionStatus[] = [
  { kind: 'microphone', state: 'granted', optional: false },
  { kind: 'system_audio', state: 'granted', optional: false },
  { kind: 'notifications', state: 'granted', optional: true },
  { kind: 'accessibility', state: 'denied', optional: true },
  { kind: 'screen_recording', state: 'not_determined', optional: true },
];

function renderCard() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PermissionsSettings />
    </QueryClientProvider>,
  );
}

describe('Settings → Recording → Permissions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bindings.onboarding.permissions.mockResolvedValue({ status: 'ok', data: STATUSES });
    bindings.calendarStatus.mockResolvedValue({
      status: 'ok',
      data: {
        access: 'granted',
        syncEnabled: true,
        lastSyncedAt: null,
        lastError: null,
        connectors: [],
      },
    });
  });

  it('shows each permission once, granted rows only as Allowed', async () => {
    renderCard();

    expect(await screen.findByText('Accessibility')).toBeTruthy();
    expect(screen.getByText('Screen Recording')).toBeTruthy();
    expect(await screen.findByText('Calendar')).toBeTruthy();
    expect(screen.getAllByText('Allowed')).toHaveLength(4);
    expect(screen.queryByText('Optional')).toBeNull();
  });

  it('opens System Settings for a refused permission, and asks for an unasked one', async () => {
    bindings.onboarding.openPermissionSettings.mockResolvedValue({ status: 'ok', data: null });
    renderCard();

    fireEvent.click(await screen.findByRole('button', { name: 'Open System Settings' }));
    fireEvent.click(screen.getByRole('button', { name: 'Allow' }));

    await waitFor(() => {
      expect(bindings.onboarding.openPermissionSettings).toHaveBeenCalledWith('accessibility');
    });
    expect(bindings.onboarding.requestPermission).toHaveBeenCalledWith('screen_recording');
  });
});
