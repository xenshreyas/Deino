import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
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

/** The section at `/settings`, so its link can navigate. */
function renderPage() {
  const root = createRootRoute();
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
    component: SpeakerTagsSettings,
  });
  const router = createRouter({
    routeTree: root.addChildren([settings]),
    history: createMemoryHistory({ initialEntries: ['/settings'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

describe('SpeakerTagsSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('lists the apps as plain text, with no row or button of its own', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: rows('enabled') });
    renderPage();

    expect(
      await screen.findByText('Works with Zoom, Microsoft Teams and Google Meet.'),
    ).toBeTruthy();
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByRole('link')).toBeNull();
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
      ),
    ).toBeTruthy();
  });

  it('links once to the Permissions card while Accessibility is missing', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: rows('needsPermission') });
    const router = renderPage();

    fireEvent.click(await screen.findByRole('link', { name: 'Needs Accessibility' }));

    await waitFor(() => {
      expect(router.state.location.search).toEqual({ section: 'permissions' });
    });
    expect(mocked.openAccessibilitySettings).not.toHaveBeenCalled();
  });

  it('says so when no app is supported here', async () => {
    mocked.status.mockResolvedValue({ status: 'ok', data: rows('unsupported') });
    renderPage();

    expect(await screen.findByText(/Not available on this Mac/)).toBeTruthy();
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
