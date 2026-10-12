import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { SidebarNav } from './sidebar-nav';

vi.mock('@/features/folders', () => ({
  FolderNavSection: () => <div role="tree" aria-label="Folders" />,
}));

const PATHS = [
  '/',
  '/meetings',
  '/action-items',
  '/people',
  '/insights',
  '/recipes',
  '/automations',
];

/** The nav inside a router that knows its destinations, opened at `at`. */
function renderNav(at = '/') {
  const root = createRootRoute({
    component: () => (
      <>
        <SidebarNav />
        <Outlet />
      </>
    ),
  });
  const pages = PATHS.map((path) =>
    createRoute({ getParentRoute: () => root, path, component: () => null }),
  );
  const router = createRouter({
    routeTree: root.addChildren(pages),
    history: createMemoryHistory({ initialEntries: [at] }),
  });
  render(<RouterProvider router={router} />);
}

describe('the sidebar (UX-25)', () => {
  it('shows four links and the Folders tree, with no section labels', async () => {
    renderNav();

    const nav = await screen.findByRole('navigation', { name: 'Main' });
    const links = within(nav)
      .getAllByRole('link')
      .map((link) => link.textContent);
    expect(links).toEqual(['Home', 'Meetings', 'Action items', 'People']);
    expect(within(nav).getByRole('tree', { name: 'Folders' })).toBeTruthy();
    expect(within(nav).queryByRole('heading')).toBeNull();
  });

  it('keeps Insights, Recipes and Automations under a closed More', async () => {
    renderNav();

    const more = await screen.findByRole('button', { name: 'More' });
    expect(more.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('link', { name: 'Insights' })).toBeNull();

    fireEvent.click(more);

    expect(more.getAttribute('aria-expanded')).toBe('true');
    for (const name of ['Insights', 'Recipes', 'Automations']) {
      expect(screen.getByRole('link', { name })).toBeTruthy();
    }
  });

  it('opens More while one of its pages is showing', async () => {
    renderNav('/recipes');

    expect(await screen.findByRole('link', { name: 'Recipes' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'More' }).getAttribute('aria-expanded')).toBe('true');
  });
});
