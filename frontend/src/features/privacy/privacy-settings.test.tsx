import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { defaultSettings } from '../settings/settings-fixture';
import type * as Privacy from '@/lib/ipc/generated/privacy';
import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { encryptionNote } from './components/encryption-settings';
import { PrivacySettings } from './components/privacy-settings';
import { privacyCopy } from './copy';

const settings = vi.hoisted(() => ({
  get: vi.fn<typeof Generated.commands.get>(),
  patch: vi.fn<typeof Generated.commands.patch>(),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({ commands: settings }));

const privacy = vi.hoisted(() => ({
  state: vi.fn<typeof Privacy.commands.state>(),
  touch: vi.fn<typeof Privacy.commands.touch>(),
  lock: vi.fn<typeof Privacy.commands.lock>(),
  unlock: vi.fn<typeof Privacy.commands.unlock>(),
  library: vi.fn<typeof Privacy.commands.library>(),
}));
vi.mock('@/lib/ipc/generated/privacy', () => ({ commands: privacy }));

const copy = privacyCopy.settings;
const PLAIN: Privacy.LibraryStatus = {
  kind: 'ready',
  encrypted: false,
  outcome: { kind: 'unchanged' },
};

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PrivacySettings />
    </QueryClientProvider>,
  );
}

describe('Settings → Privacy & security', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settings.get.mockResolvedValue({ status: 'ok', data: defaultSettings() });
    settings.patch.mockImplementation(() =>
      Promise.resolve({ status: 'ok', data: defaultSettings() }),
    );
    privacy.library.mockResolvedValue({ status: 'ok', data: PLAIN });
    privacy.lock.mockResolvedValue({ status: 'ok', data: 'locked' });
  });

  test('encrypting asks first and saves only on confirm', async () => {
    renderPage();

    fireEvent.click(await screen.findByRole('switch', { name: copy.encryptTitle }));
    expect(await screen.findByText(copy.confirmEncryptTitle)).toBeDefined();
    expect(settings.patch).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: copy.confirmEncryptAction }));

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ privacy: { encryptDatabase: true } });
    });
  });

  test('cancelling the confirmation saves nothing', async () => {
    renderPage();

    fireEvent.click(await screen.findByRole('switch', { name: copy.encryptTitle }));
    fireEvent.click(await screen.findByRole('button', { name: copy.confirmCancel }));

    await waitFor(() => {
      expect(screen.queryByText(copy.confirmEncryptTitle)).toBeNull();
    });
    expect(settings.patch).not.toHaveBeenCalled();
  });

  test('turning the lock on shows its options, and Lock now locks', async () => {
    const on = defaultSettings();
    on.privacy.appLock.enabled = true;
    settings.get.mockResolvedValue({ status: 'ok', data: on });
    renderPage();

    expect(await screen.findByRole('combobox', { name: copy.idleTitle })).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: copy.lockNow }));

    await waitFor(() => {
      expect(privacy.lock).toHaveBeenCalled();
    });
  });

  test('the lock switch saves a merge patch', async () => {
    renderPage();

    fireEvent.click(await screen.findByRole('switch', { name: copy.lockSwitch }));

    await waitFor(() => {
      expect(settings.patch).toHaveBeenCalledWith({ privacy: { appLock: { enabled: true } } });
    });
  });
});

describe('encryptionNote', () => {
  test('asks for a restart while the setting and the file differ', () => {
    expect(encryptionNote(true, PLAIN)).toBe(copy.restartToEncrypt);
    expect(encryptionNote(false, { ...PLAIN, encrypted: true })).toBe(copy.restartToDecrypt);
    expect(encryptionNote(false, PLAIN)).toBe(copy.plaintextNow);
  });

  test('explains a failed conversion', () => {
    const failed: Privacy.LibraryStatus = {
      kind: 'ready',
      encrypted: false,
      outcome: { kind: 'failed', direction: 'encrypt' },
    };

    expect(encryptionNote(true, failed)).toBe(copy.failed('encrypt'));
  });
});
