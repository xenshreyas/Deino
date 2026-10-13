import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/settings';
import { createQueryClient } from '@/lib/ipc';

import { notificationsCopy as copy } from '../copy';
import { defaultSettings } from '../settings-fixture';
import { AutoNotesSettings } from './auto-notes-settings';
import { NotificationsSettings } from './notifications-settings';

// Typed against the generated bindings: a Rust signature change breaks these mocks.
const bindings = vi.hoisted(() => ({
  settings: {
    get: vi.fn<typeof Generated.commands.get>(),
    patch: vi.fn<typeof Generated.commands.patch>(),
  } satisfies typeof Generated.commands,
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));
vi.mock('@/lib/ipc/generated/settings', () => ({
  commands: bindings.settings,
  events: { settingsChanged: { listen: bindings.listen } },
}));

function renderSections() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <NotificationsSettings />
      <AutoNotesSettings />
    </QueryClientProvider>,
  );
}

describe('Settings → General → Notifications', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const settings = defaultSettings();
    bindings.settings.get.mockResolvedValue({ status: 'ok', data: settings });
    bindings.settings.patch.mockResolvedValue({ status: 'ok', data: settings });
  });

  it('lists one switch per kind of notification', async () => {
    renderSections();

    expect(await screen.findByRole('switch', { name: copy.recordingTitle })).toBeTruthy();
    expect(screen.getByRole('switch', { name: copy.remindersTitle })).toBeTruthy();
    expect(screen.getByRole('switch', { name: copy.actionItemsTitle })).toBeTruthy();
    expect(screen.getByRole('button', { name: copy.moreLabel })).toBeTruthy();
  });

  it('turns both recording notifications on together', async () => {
    renderSections();

    fireEvent.click(await screen.findByRole('switch', { name: copy.recordingTitle }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        notifications: { preferences: { showRecordingStarted: true, showRecordingStopped: true } },
      });
    });
  });

  it('keeps each kind under its own settings key', async () => {
    renderSections();

    fireEvent.click(await screen.findByRole('switch', { name: copy.remindersTitle }));
    fireEvent.click(screen.getByRole('switch', { name: copy.actionItemsTitle }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        calendar: { reminders: { enabled: false } },
      });
    });
    expect(bindings.settings.patch).toHaveBeenCalledWith({
      notifications: { preferences: { showActionItemReminders: false } },
    });
  });

  it('turns the prompt and reminder chime off', async () => {
    renderSections();

    fireEvent.click(await screen.findByRole('switch', { name: copy.soundTitle }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({
        notifications: { notificationSound: false },
      });
    });
  });

  it('keeps automatic notes reachable in Advanced', async () => {
    renderSections();

    // On by default; the switch turns it off.
    fireEvent.click(await screen.findByRole('switch', { name: 'Write notes after each meeting' }));

    await waitFor(() => {
      expect(bindings.settings.patch).toHaveBeenCalledWith({ ai: { autoSummary: false } });
    });
  });
});
