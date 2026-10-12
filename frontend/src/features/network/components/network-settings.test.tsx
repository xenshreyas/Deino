import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/network';
import { DEFAULT_SETTINGS } from '@/lib/ipc/generated/settings-defaults';
import { createQueryClient } from '@/lib/ipc';

import { NetworkSettings } from './network-settings';
import { ServicesSection, servicesInUse } from './service-list';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const network = vi.hoisted(() => ({
  services: vi.fn<typeof Generated.commands.services>(),
  setServiceEnabled: vi.fn<typeof Generated.commands.setServiceEnabled>(),
  egressLog: vi.fn<typeof Generated.commands.egressLog>(),
}));
vi.mock('@/lib/ipc/generated/network', () => ({ commands: network }));

// Slack has a saved switch (off); Groq was never set up.
const settings = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings, events: {} }));

const ANTHROPIC: Generated.ServiceStatus = {
  id: 'llm_anthropic',
  kind: 'llm',
  hosts: ['api.anthropic.com'],
  enabled: true,
};
const SLACK: Generated.ServiceStatus = {
  id: 'slack',
  kind: 'integration',
  hosts: ['slack.com', '*.slack.com'],
  enabled: false,
};
const GROQ: Generated.ServiceStatus = {
  id: 'llm_groq',
  kind: 'llm',
  hosts: ['api.groq.com'],
  enabled: true,
};
const REQUEST: Generated.EgressEntry = {
  at: '2026-10-09T09:30:05Z',
  service: 'llm_anthropic',
  host: 'api.anthropic.com',
  method: 'POST',
  status: 200,
  outcome: 'sent',
  bytesUp: 48_213,
  bytesDown: 512,
};

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;
}

function renderPage(onOfflineModeChange = vi.fn()) {
  render(<NetworkSettings offlineMode={false} onOfflineModeChange={onOfflineModeChange} />, {
    wrapper,
  });
  return onOfflineModeChange;
}

function renderServices() {
  render(<ServicesSection />, { wrapper });
}

describe('servicesInUse', () => {
  it('keeps services that sent something or have a saved switch', () => {
    expect(servicesInUse([ANTHROPIC, SLACK, GROQ], [REQUEST], ['slack'])).toEqual([
      ANTHROPIC,
      SLACK,
    ]);
    expect(servicesInUse([SLACK, GROQ], [], [])).toEqual([]);
  });
});

describe('Network services', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const saved = structuredClone(DEFAULT_SETTINGS);
    settings.get.mockResolvedValue({
      status: 'ok',
      data: { ...saved, network: { ...saved.network, services: { slack: false } } },
    });
    network.services.mockResolvedValue({ status: 'ok', data: [ANTHROPIC, SLACK, GROQ] });
    network.egressLog.mockResolvedValue({ status: 'ok', data: [REQUEST] });
  });

  it('shows only the services in use until all are asked for', async () => {
    renderServices();

    expect(await screen.findByRole('switch', { name: 'Allow Anthropic' })).toBeTruthy();
    expect(await screen.findByRole('switch', { name: 'Allow Slack' })).toBeTruthy();
    expect(screen.queryByRole('switch', { name: 'Allow Groq' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Show all 3 services' }));

    expect(screen.getByRole('switch', { name: 'Allow Groq' })).toBeTruthy();
  });

  it('groups services by kind, with their hosts and switches', async () => {
    renderServices();

    const providers = await screen.findByRole('group', { name: 'AI providers' });
    expect(within(providers).getByText('api.anthropic.com')).toBeTruthy();
    expect(
      screen.getByRole('switch', { name: 'Allow Anthropic' }).getAttribute('aria-checked'),
    ).toBe('true');
    const apps = screen.getByRole('group', { name: 'Connected apps' });
    expect(within(apps).getByText('slack.com, *.slack.com')).toBeTruthy();
  });

  it('switching a service on saves it and shows the saved state', async () => {
    network.setServiceEnabled.mockResolvedValue({
      status: 'ok',
      data: { ...SLACK, enabled: true },
    });
    renderServices();

    fireEvent.click(await screen.findByRole('switch', { name: 'Allow Slack' }));

    await waitFor(() => {
      const slack = screen.getByRole('switch', { name: 'Allow Slack' });
      expect(slack.getAttribute('aria-checked')).toBe('true');
    });
    expect(network.setServiceEnabled).toHaveBeenCalledWith('slack', true);
  });
});

describe('NetworkSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    network.egressLog.mockResolvedValue({ status: 'ok', data: [REQUEST] });
  });

  it('shows offline mode and the activity log, not the services', async () => {
    renderPage();

    expect(await screen.findByText('What left this Mac')).toBeTruthy();
    expect(screen.queryByText('Network services')).toBeNull();
    expect(network.services).not.toHaveBeenCalled();
  });

  it('lists each request with its host, status and sizes', async () => {
    renderPage();

    const row = (await screen.findByText('POST 200')).closest('tr');
    expect(row).not.toBeNull();
    const cells = within(row ?? document.body);
    expect(cells.getByText('Anthropic')).toBeTruthy();
    expect(cells.getByText('47 KB / 512 B')).toBeTruthy();
    expect(network.egressLog).toHaveBeenCalledWith(100);
  });

  it('shows a calm line when nothing has left the Mac', async () => {
    network.egressLog.mockResolvedValue({ status: 'ok', data: [] });
    renderPage();

    expect(await screen.findByText('Nothing has left this Mac yet.')).toBeTruthy();
  });

  it('offline mode is a switch that reports the new value', () => {
    const onOfflineModeChange = renderPage();

    fireEvent.click(screen.getByRole('switch', { name: 'Offline mode' }));

    expect(onOfflineModeChange).toHaveBeenCalledWith(true);
  });
});
