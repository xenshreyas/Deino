import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { AppTagStatus, TagState } from '@/lib/ipc/generated/speaker-tags';
import { createQueryClient } from '@/lib/ipc';

import { SpeakerTagsSettings } from './speaker-tags-settings';

const bindings = vi.hoisted(() => ({
  commands: {
    status: vi.fn(),
    openAccessibilitySettings: vi.fn(),
  },
}));
vi.mock('@/lib/ipc/generated/speaker-tags', () => bindings);
const mocked = bindings.commands;

/** Every app in `state`. */
function rows(state: TagState): AppTagStatus[] {
  return (['zoom', 'teams', 'meet'] as const).map((app) => ({ app, state }));
}

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpeakerTagsSettings />
    </QueryClientProvider>,
  );
}

describe('SpeakerTagsSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows one Accessibility row and the apps as plain text', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: rows('enabled') });
    renderPage();

    expect(await screen.findByText('Accessibility')).toBeTruthy();
    expect(screen.getByText('Allowed')).toBeTruthy();
    expect(
      screen.getByText('Works with Zoom, Microsoft Teams and Google Meet.', { exact: false }),
    ).toBeTruthy();
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('marks an app only when it is unsupported', async () => {
    mocked.status.mockResolvedValue({
      status: 'ok',
      data: [
        { app: 'zoom', state: 'enabled' },
        { app: 'teams', state: 'unsupported' },
        { app: 'meet', state: 'enabled' },
      ],
    });
    renderPage();

    expect(
      await screen.findByText(
        'Works with Zoom, Microsoft Teams (not supported here) and Google Meet.',
        { exact: false },
      ),
    ).toBeTruthy();
    expect(screen.getByText('Allowed')).toBeTruthy();
  });

  it('offers System Settings once, without asking macOS itself', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: rows('needsPermission') });
    mocked.openAccessibilitySettings.mockResolvedValue({ status: 'ok', data: null });
    renderPage();

    const open = await screen.findByRole('button', {
      name: 'Open System Settings to allow Accessibility',
    });
    expect(screen.getAllByRole('button')).toHaveLength(1);
    expect(screen.getAllByText('Needs permission')).toHaveLength(1);
    fireEvent.click(open);

    await waitFor(() => {
      expect(mocked.openAccessibilitySettings).toHaveBeenCalledOnce();
    });
  });

  it('says so inline when System Settings would not open', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: rows('needsPermission') });
    mocked.openAccessibilitySettings.mockResolvedValue({
      status: 'error',
      error: { code: 'unavailable', message: 'System Settings could not be opened' },
    });
    renderPage();

    fireEvent.click(
      await screen.findByRole('button', { name: 'Open System Settings to allow Accessibility' }),
    );

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Open Privacy & Security → Accessibility there.',
    );
  });

  it('shows an inline error when the status cannot be read', async () => {
    mocked.status.mockResolvedValue({
      status: 'error',
      error: { code: 'internal', message: 'boom' },
    });
    renderPage();

    expect((await screen.findByRole('alert')).textContent).toBe(
      'Couldn’t read the speaker-name status.',
    );
  });
});
