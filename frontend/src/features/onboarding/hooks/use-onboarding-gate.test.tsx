import { QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { FRESH } from '../machine';
import { useOnboardingGate } from './use-onboarding-gate';

const get = vi.hoisted(() => vi.fn<typeof Generated.commands.get>());
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: { get, patch: vi.fn() },
  events: {},
}));

function gateWith(completed: boolean | 'error') {
  if (completed === 'error') {
    get.mockResolvedValue({ status: 'error', error: { code: 'internal', message: 'unreadable' } });
  } else {
    const onboarding = { ...FRESH, completed };
    // The gate reads only `onboarding`; a partial document is enough.
    get.mockResolvedValue({ status: 'ok', data: { onboarding } as Generated.Settings });
  }
  const client = createQueryClient();
  client.setDefaultOptions({ queries: { retry: false } });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(() => useOnboardingGate(), { wrapper }).result;
}

describe('useOnboardingGate', () => {
  it('waits for the settings before showing anything', () => {
    expect(gateWith(false).current).toBe('pending');
  });

  it('routes a fresh install into onboarding', async () => {
    const gate = gateWith(false);

    await waitFor(() => {
      expect(gate.current).toBe('onboarding');
    });
  });

  it('never shows onboarding again once it is completed', async () => {
    const gate = gateWith(true);

    await waitFor(() => {
      expect(gate.current).toBe('app');
    });
  });

  it('opens the app when the settings cannot be read', async () => {
    const gate = gateWith('error');

    await waitFor(() => {
      expect(gate.current).toBe('app');
    });
  });
});
