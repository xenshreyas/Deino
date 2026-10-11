import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/meetings';
import { createQueryClient } from '@/lib/ipc';

import { useShellLayout } from '../layout-store';
import { CommandPalette } from './command-palette';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      get: vi.fn<typeof Generated.commands.get>(),
      rename: vi.fn<typeof Generated.commands.rename>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/meetings', () => ({ commands }));
vi.mock('@/contexts/ConfigContext', () => ({
  useConfig: () => ({ betaFeatures: { importAndRetranscribe: false } }),
}));
vi.mock('@/contexts/ImportDialogContext', () => ({
  useImportDialog: () => ({ openImportDialog: vi.fn() }),
}));

/** A router with the palette in its root and two pages to land on. */
function renderPalette() {
  const root = createRootRoute({
    component: () => (
      <>
        <CommandPalette />
        <Outlet />
      </>
    ),
  });
  const page = (path: string) =>
    createRoute({ getParentRoute: () => root, path, component: () => null });
  const router = createRouter({
    routeTree: root.addChildren([page('/'), page('/meetings'), page('/meetings/$meetingId')]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

describe('command palette', () => {
  beforeEach(() => {
    commands.list.mockResolvedValue({
      status: 'ok',
      data: [
        {
          id: 'meeting-a' as Generated.MeetingId,
          title: 'Q4 roadmap alignment',
          createdAt: '2026-10-08T14:00:00Z',
          updatedAt: '2026-10-08T14:00:00Z',
        },
      ],
    });
  });

  test('opens from the shell store with navigation, actions and recent meetings', async () => {
    renderPalette();
    act(() => {
      useShellLayout.getState().setPaletteOpen(true);
    });

    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeDefined();
    expect(screen.getByText('Go to')).toBeDefined();
    expect(screen.getByText('Actions')).toBeDefined();
    expect(await screen.findByText('Q4 roadmap alignment')).toBeDefined();
  });

  test('choosing a destination navigates there and closes the palette', async () => {
    const router = renderPalette();
    act(() => {
      useShellLayout.getState().setPaletteOpen(true);
    });

    fireEvent.click(await screen.findByRole('option', { name: 'Meetings' }));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/meetings');
    });
    expect(useShellLayout.getState().paletteOpen).toBe(false);
  });

  test('a meeting result opens that meeting', async () => {
    const router = renderPalette();
    act(() => {
      useShellLayout.getState().setPaletteOpen(true);
    });

    fireEvent.click(await screen.findByRole('option', { name: /Q4 roadmap alignment/ }));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/meetings/meeting-a');
    });
  });
});
