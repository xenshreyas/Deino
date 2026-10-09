import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Secrets from '@/lib/ipc/generated/secrets';
import type * as SettingsBindings from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { defaultSettings } from '../../settings/settings-fixture';
import { connector, status } from '../test-calendar';
import { ConnectorsSection } from './connectors-section';

const bindings = await vi.hoisted(async () =>
  (await import('../test-calendar')).calendarBindings(),
);
// Typed against the generated bindings: a Rust signature change breaks these mocks.
const settingsBindings = vi.hoisted(() => ({
  settings: {
    get: vi.fn<typeof SettingsBindings.commands.get>(),
    patch: vi.fn<typeof SettingsBindings.commands.patch>(),
  } satisfies typeof SettingsBindings.commands,
  secrets: {
    set: vi.fn<typeof Secrets.commands.set>(),
    status: vi.fn<typeof Secrets.commands.status>(),
    delete: vi.fn<typeof Secrets.commands.delete>(),
  } satisfies typeof Secrets.commands,
}));
vi.mock('@/lib/ipc/generated/calendar', () => bindings);
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settingsBindings.settings }));
vi.mock('@/lib/ipc/generated/secrets', () => ({ commands: settingsBindings.secrets }));
const mocked = bindings.commands;
const { settings, secrets } = settingsBindings;

function renderSection() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ConnectorsSection />
    </QueryClientProvider>,
  );
}

/** The card of the connector named `name`. */
async function card(name: string) {
  return within(await screen.findByRole('group', { name }));
}

describe('ConnectorsSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
    settings.patch.mockResolvedValue({ status: 'ok', data: defaultSettings() });
    secrets.status.mockResolvedValue({ status: 'ok', data: 'missing' });
    secrets.set.mockResolvedValue({ status: 'ok', data: null });
  });

  it('shows nothing when the app has no direct connectors', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: status() });
    renderSection();

    await waitFor(() => {
      expect(mocked.status).toHaveBeenCalled();
    });
    expect(screen.queryByText('Direct connections')).toBeNull();
  });

  it('asks for the app registration, then saves the client ID and the secret', async () => {
    mocked.status.mockResolvedValue({
      status: 'ok',
      data: status({ connectors: [connector('google', { configured: false })] }),
    });
    renderSection();
    const google = await card('Google Calendar');

    expect(google.getByText('Add your app registration to connect.')).toBeTruthy();
    expect(google.getByRole('button', { name: 'Connect' }).hasAttribute('disabled')).toBe(true);
    const clientId = google.getByLabelText('Client ID');
    fireEvent.change(clientId, { target: { value: ' 123.apps.googleusercontent.com ' } });
    fireEvent.blur(clientId);
    const secret = google.getByLabelText('Client secret');
    fireEvent.change(secret, { target: { value: 'GOCSPX-secret' } });
    fireEvent.blur(secret);

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({
        calendar: { google: { clientId: '123.apps.googleusercontent.com' } },
      });
    });
    expect(secrets.set).toHaveBeenCalledWith('google_oauth_client_secret', 'GOCSPX-secret');
    expect((secret as HTMLInputElement).value).toBe('');
    // Whether the connector is set up comes from the status: it is read again.
    await waitFor(() => {
      expect(mocked.status).toHaveBeenCalledTimes(2);
    });
  });

  it('connects a set-up connector, then disconnects it', async () => {
    mocked.status.mockResolvedValue({
      status: 'ok',
      data: status({ connectors: [connector('microsoft')] }),
    });
    const connected = connector('microsoft', {
      connected: true,
      lastSyncedAt: new Date().toISOString(),
    });
    mocked.connect.mockResolvedValue({ status: 'ok', data: status({ connectors: [connected] }) });
    mocked.disconnect.mockResolvedValue({
      status: 'ok',
      data: status({ connectors: [connector('microsoft')] }),
    });
    renderSection();
    const outlook = await card('Microsoft Outlook');

    expect(outlook.queryByLabelText('Client secret')).toBeNull();
    fireEvent.click(outlook.getByRole('button', { name: 'Connect' }));
    expect(await outlook.findByText('Connected · Synced just now')).toBeTruthy();
    fireEvent.click(outlook.getByRole('button', { name: 'Disconnect' }));

    expect(await outlook.findByText('Not connected')).toBeTruthy();
    expect(mocked.connect).toHaveBeenCalledWith('microsoft');
    expect(mocked.disconnect).toHaveBeenCalledWith('microsoft');
  });

  it('says so when the sign-in fails, and shows the last sync error', async () => {
    mocked.status.mockResolvedValue({
      status: 'ok',
      data: status({
        connectors: [connector('microsoft', { lastError: 'Microsoft refused the sign-in.' })],
      }),
    });
    mocked.connect.mockResolvedValue({
      status: 'error',
      error: { code: 'permission_denied', message: 'the sign-in was declined' },
    });
    renderSection();
    const outlook = await card('Microsoft Outlook');

    expect(outlook.getByRole('alert').textContent).toBe('Microsoft refused the sign-in.');
    fireEvent.click(outlook.getByRole('button', { name: 'Connect' }));

    expect(
      await screen.findByText('Could not connect. Check the app registration and try again.'),
    ).toBeTruthy();
  });
});
