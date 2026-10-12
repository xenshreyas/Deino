import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Onboarding from '@/lib/ipc/generated/onboarding';

const mocks = vi.hoisted(() => ({
  permissions: vi.fn<typeof Onboarding.commands.permissions>(),
  requestPermission: vi.fn<typeof Onboarding.commands.requestPermission>(),
  toast: vi.fn(),
}));
vi.mock('@/lib/ipc/generated/onboarding', () => ({
  commands: { permissions: mocks.permissions, requestPermission: mocks.requestPermission },
}));
vi.mock('sonner', () => ({ toast: mocks.toast }));

/** A fresh copy of the module, so its once-per-launch memory starts empty. */
async function freshAsk() {
  vi.resetModules();
  return (await import('./ask-in-context')).askPermissionOnce;
}

function accessibility(state: Onboarding.PermissionState) {
  mocks.permissions.mockResolvedValue({
    status: 'ok',
    data: [{ kind: 'accessibility', state, optional: true }],
  });
}

describe('asking for a permission where it matters (UX-22)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('offers Accessibility once, with an Allow button that asks macOS', async () => {
    accessibility('not_determined');
    mocks.requestPermission.mockResolvedValue({
      status: 'ok',
      data: { kind: 'accessibility', state: 'granted', optional: true },
    });
    const ask = await freshAsk();

    await ask('accessibility');
    await ask('accessibility');

    expect(mocks.toast).toHaveBeenCalledTimes(1);
    const [title, options] = mocks.toast.mock.calls[0] as [
      string,
      { action: { label: string; onClick: () => void } },
    ];
    expect(title).toBe('Show who is speaking');
    expect(options.action.label).toBe('Allow');
    options.action.onClick();
    expect(mocks.requestPermission).toHaveBeenCalledWith('accessibility');
  });

  it('says nothing once it is allowed or refused', async () => {
    for (const state of ['granted', 'denied'] as const) {
      accessibility(state);
      const ask = await freshAsk();

      await ask('accessibility');
    }

    expect(mocks.toast).not.toHaveBeenCalled();
  });
});
