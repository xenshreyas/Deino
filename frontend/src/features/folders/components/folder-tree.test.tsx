import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/folders';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { folderEntry } from '../test-folders';
import { useFolderTreeStore } from '../tree-store';
import { FolderNavSection } from './folder-tree';

const commands = vi.hoisted(
  () =>
    ({
      tree: vi.fn<typeof Generated.commands.tree>(),
      get: vi.fn<typeof Generated.commands.get>(),
      create: vi.fn<typeof Generated.commands.create>(),
      rename: vi.fn<typeof Generated.commands.rename>(),
      setIcon: vi.fn<typeof Generated.commands.setIcon>(),
      setDescription: vi.fn<typeof Generated.commands.setDescription>(),
      moveTo: vi.fn<typeof Generated.commands.moveTo>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      meetings: vi.fn<typeof Generated.commands.meetings>(),
      unfiled: vi.fn<typeof Generated.commands.unfiled>(),
      foldersOf: vi.fn<typeof Generated.commands.foldersOf>(),
      addMeeting: vi.fn<typeof Generated.commands.addMeeting>(),
      removeMeeting: vi.fn<typeof Generated.commands.removeMeeting>(),
      rules: vi.fn<typeof Generated.commands.rules>(),
      addRule: vi.fn<typeof Generated.commands.addRule>(),
      removeRule: vi.fn<typeof Generated.commands.removeRule>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/folders', () => ({ commands }));

// Work / Clients, and Home.
const work = folderEntry('work', 'Work', null, 0);
const clients = folderEntry('clients', 'Clients', 'work', 1);
const home = folderEntry('home', 'Home', null, 0);

/** The section in a router with the folder routes it links to. */
function renderTree() {
  const root = createRootRoute({ component: FolderNavSection });
  const pages = ['/folders', '/folders/$folderId', '/folders/unfiled'].map((path) =>
    createRoute({ getParentRoute: () => root, path, component: () => null }),
  );
  const router = createRouter({
    routeTree: root.addChildren(pages),
    history: createMemoryHistory({ initialEntries: ['/folders'] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

/** A `DataTransfer` that carries `data`, as a drop event's does. */
function dropping(data: Record<string, string>) {
  return {
    dataTransfer: {
      types: Object.keys(data),
      getData: (format: string) => data[format] ?? '',
      dropEffect: 'none',
    },
  };
}

const row = (name: string) => screen.getByRole('treeitem', { name });

describe('the sidebar folder tree', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    useFolderTreeStore.setState({ open: true, collapsed: new Set() });
    commands.tree.mockResolvedValue({
      status: 'ok',
      data: { entries: [work, clients, home], unfiledCount: 3 },
    });
  });

  test('shows the folders as a tree with Unfiled, and collapses a folder', async () => {
    renderTree();

    expect(
      (await screen.findByRole('treeitem', { name: 'Clients' })).getAttribute('aria-level'),
    ).toBe('2');
    expect(row('Work').getAttribute('aria-expanded')).toBe('true');
    expect(screen.getByRole('link', { name: /Unfiled/ }).textContent).toContain('3');

    fireEvent.click(screen.getByRole('button', { name: 'Collapse Work' }));

    expect(screen.queryByRole('treeitem', { name: 'Clients' })).toBeNull();
    expect(row('Work').getAttribute('aria-expanded')).toBe('false');
  });

  test('arrow keys move the focus and open and close folders', async () => {
    renderTree();
    const workRow = await screen.findByRole('treeitem', { name: 'Work' });
    workRow.focus();

    fireEvent.keyDown(workRow, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(row('Clients'));

    fireEvent.keyDown(row('Clients'), { key: 'ArrowLeft' });
    expect(document.activeElement).toBe(row('Work'));
    fireEvent.keyDown(row('Work'), { key: 'ArrowLeft' });
    expect(screen.queryByRole('treeitem', { name: 'Clients' })).toBeNull();
  });

  test('dropping a meeting on a folder files it there', async () => {
    commands.addMeeting.mockResolvedValue({ status: 'ok', data: { seriesOffer: null } });
    renderTree();
    const target = (await screen.findByRole('treeitem', { name: 'Home' })).closest('li');
    if (target === null) throw new Error('the row has no list item');

    const drag = dropping({ 'application/x-deino-meeting': 'meeting-1' });
    fireEvent.dragOver(target, drag);
    expect(target.dataset.over).toBe('true');
    fireEvent.drop(target, drag);

    await waitFor(() => {
      expect(commands.addMeeting).toHaveBeenCalledWith(home.folder.id, 'meeting-1');
    });
    expect(target.dataset.over).toBeUndefined();
  });

  test('a folder dropped on another moves under it, but never into its own subfolder', async () => {
    commands.moveTo.mockResolvedValue({ status: 'ok', data: work.folder });
    renderTree();
    const into = (name: string) =>
      screen.getByRole('treeitem', { name }).closest('li') ?? document.body;
    await screen.findByRole('treeitem', { name: 'Home' });

    fireEvent.drop(into('Clients'), dropping({ 'application/x-deino-folder': work.folder.id }));
    fireEvent.drop(into('Home'), dropping({ 'application/x-deino-folder': work.folder.id }));

    await waitFor(() => {
      expect(commands.moveTo).toHaveBeenCalledTimes(1);
    });
    expect(commands.moveTo).toHaveBeenCalledWith(work.folder.id, home.folder.id);
  });
});
