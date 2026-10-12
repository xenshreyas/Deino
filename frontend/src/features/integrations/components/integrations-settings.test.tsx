import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/integrations';
import { createQueryClient } from '@/lib/ipc';

import { IntegrationsSettings } from './integrations-settings';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const integrations = vi.hoisted(() => ({
  providers: vi.fn<typeof Generated.commands.providers>(),
  connectToken: vi.fn<typeof Generated.commands.connectToken>(),
  disconnect: vi.fn<typeof Generated.commands.disconnect>(),
  deliveries: vi.fn<typeof Generated.commands.deliveries>(),
}));
vi.mock('@/lib/ipc/generated/integrations', () => ({ commands: integrations }));
// The calendar pointer is a router link; the page itself has no route here.
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <a href="/settings">{children}</a>,
}));

const SANDBOX: Generated.ProviderView = {
  id: 'sandbox',
  name: 'Sandbox',
  available: true,
  capabilities: ['post_message', 'create_task'],
  methods: [
    { method: 'token', label: 'Sandbox token', usernameLabel: null, helpUrl: 'https://x.invalid' },
  ],
  app: null,
  connections: [],
};

const SLACK: Generated.ProviderView = {
  id: 'slack',
  name: 'Slack',
  available: false,
  capabilities: ['post_message'],
  methods: [],
  app: null,
  connections: [],
};

const CONNECTION: Generated.Connection = {
  id: '0199c6a0-0000-7000-8000-000000000001' as Generated.ConnectionId,
  provider: 'sandbox',
  accountId: 'sandbox',
  accountLabel: 'Sandbox workspace',
  authMethod: 'token',
  status: 'needs_reconnect',
  scopes: [],
  tokenExpiresAt: null,
  connectedAt: '2026-10-11T09:00:00Z',
  updatedAt: '2026-10-11T09:00:00Z',
  lastUsedAt: null,
  lastError: {
    code: 'permission_denied',
    message: 'Sandbox no longer accepts the saved sign-in; connect again',
    at: '2026-10-11T09:05:00Z',
  },
};

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

describe('IntegrationsSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    integrations.providers.mockResolvedValue({ status: 'ok', data: [SANDBOX, SLACK] });
    integrations.deliveries.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('lists only shipped providers; planned ones stay hidden (UX-27)', async () => {
    render(<IntegrationsSettings offlineMode={false} />, { wrapper });

    expect(await screen.findByRole('group', { name: 'Sandbox' })).toBeTruthy();
    expect(screen.queryByText('Slack')).toBeNull();
    expect(screen.queryByText('WIP')).toBeNull();
    expect(screen.getByText('Nothing sent yet.')).toBeTruthy();
  });

  it('connects a shipped provider with a pasted token', async () => {
    integrations.connectToken.mockResolvedValue({ status: 'ok', data: CONNECTION });
    render(<IntegrationsSettings offlineMode={false} />, { wrapper });

    const card = await screen.findByRole('group', { name: 'Sandbox' });
    fireEvent.click(within(card).getByRole('button', { name: 'Connect' }));
    fireEvent.change(within(card).getByLabelText('Sandbox token'), {
      target: { value: 'tok-1' },
    });
    fireEvent.click(within(card).getByRole('button', { name: 'Connect' }));

    await waitFor(() => {
      expect(integrations.connectToken).toHaveBeenCalledWith('sandbox', {
        token: 'tok-1',
        username: null,
      });
    });
    await waitFor(() => {
      expect(within(card).queryByLabelText('Sandbox token')).toBeNull();
    });
  });

  it('shows a connection that needs reconnecting, with its error, and disconnects it', async () => {
    integrations.providers.mockResolvedValue({
      status: 'ok',
      data: [{ ...SANDBOX, connections: [CONNECTION] }],
    });
    integrations.disconnect.mockResolvedValue({ status: 'ok', data: null });
    render(<IntegrationsSettings offlineMode={false} />, { wrapper });

    expect(await screen.findByText('Sandbox workspace')).toBeTruthy();
    expect(screen.getByText('Needs reconnecting')).toBeTruthy();
    expect(screen.getByRole('alert').textContent).toContain('connect again');
    fireEvent.click(screen.getByRole('button', { name: 'Disconnect' }));

    await waitFor(() => {
      expect(integrations.disconnect).toHaveBeenCalledWith(CONNECTION.id);
    });
  });

  it('says that offline mode stops integrations', async () => {
    render(<IntegrationsSettings offlineMode />, { wrapper });

    expect((await screen.findByRole('status')).textContent).toContain('Offline mode is on');
  });

  it('shows a rejected token next to the form', async () => {
    integrations.connectToken.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'paste the sandbox token' },
    });
    render(<IntegrationsSettings offlineMode={false} />, { wrapper });

    const card = await screen.findByRole('group', { name: 'Sandbox' });
    fireEvent.click(within(card).getByRole('button', { name: 'Connect' }));
    fireEvent.change(within(card).getByLabelText('Sandbox token'), { target: { value: 'x' } });
    fireEvent.click(within(card).getByRole('button', { name: 'Connect' }));

    expect((await within(card).findByRole('alert')).textContent).toContain(
      'paste the sandbox token',
    );
  });
});
