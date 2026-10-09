import { describe, expect, it } from 'vitest';

import { folderEntry } from './test-folders';
import {
  canMoveFolder,
  folderPath,
  hasChildren,
  moveTargets,
  parentIds,
  subtreeIds,
  visibleEntries,
} from './tree';
import { treeKeyEffect } from './tree-keys';
import type { FolderId } from './types';

// Work / Clients / Acme, Work / Admin, Home: in display order, as Rust sends it.
const work = folderEntry('work', 'Work', null, 0);
const clients = folderEntry('clients', 'Clients', 'work', 1);
const acme = folderEntry('acme', 'Acme', 'clients', 2);
const admin = folderEntry('admin', 'Admin', 'work', 1);
const home = folderEntry('home', 'Home', null, 0);
const entries = [work, clients, acme, admin, home];
const id = (entry: typeof work) => entry.folder.id;
const names = (list: readonly (typeof work)[]) => list.map((entry) => entry.folder.name);

describe('the folder tree', () => {
  it('hides the descendants of collapsed folders and nothing else', () => {
    expect(names(visibleEntries(entries, new Set([id(work)])))).toEqual(['Work', 'Home']);
    expect(names(visibleEntries(entries, new Set([id(clients)])))).toEqual([
      'Work',
      'Clients',
      'Admin',
      'Home',
    ]);
    expect(visibleEntries(entries, new Set())).toHaveLength(5);
  });

  it('knows which folders have subfolders', () => {
    expect([...parentIds(entries)].sort()).toEqual([id(clients), id(work)].sort());
    expect(hasChildren(entries, 0)).toBe(true);
    expect(hasChildren(entries, 3)).toBe(false);
  });

  it('collects a folder and everything below it', () => {
    expect(subtreeIds(entries, id(work))).toEqual(
      new Set([id(work), id(clients), id(acme), id(admin)]),
    );
    expect(subtreeIds(entries, id(home))).toEqual(new Set([id(home)]));
    expect(subtreeIds(entries, 'unknown' as FolderId).size).toBe(0);
  });

  it('never lets a folder move into itself, its subfolders or where it already is', () => {
    expect(canMoveFolder(entries, id(work), id(acme))).toBe(false);
    expect(canMoveFolder(entries, id(work), id(work))).toBe(false);
    expect(canMoveFolder(entries, id(clients), id(work))).toBe(false);
    expect(canMoveFolder(entries, id(clients), id(home))).toBe(true);
    expect(canMoveFolder(entries, id(clients), null)).toBe(true);
    expect(canMoveFolder(entries, id(home), null)).toBe(false);
    expect(names(moveTargets(entries, id(clients)))).toEqual(['Admin', 'Home']);
  });

  it('spells a folder’s path from the top', () => {
    expect(folderPath(entries, id(acme))).toEqual(['Work', 'Clients', 'Acme']);
    expect(folderPath(entries, id(admin))).toEqual(['Work', 'Admin']);
    expect(folderPath(entries, 'unknown' as FolderId)).toEqual([]);
  });
});

describe('arrow keys in the tree', () => {
  const tree = { collapsed: new Set<FolderId>(), parents: parentIds(entries) };
  const press = (current: typeof work, key: string, state = tree, visible = entries) =>
    treeKeyEffect(visible, state, id(current), key);

  it('move up and down the visible rows and jump to the ends', () => {
    expect(press(work, 'ArrowDown')).toEqual({ focus: id(clients) });
    expect(press(clients, 'ArrowUp')).toEqual({ focus: id(work) });
    expect(press(work, 'ArrowUp')).toBeNull();
    expect(press(acme, 'End')).toEqual({ focus: id(home) });
    expect(press(acme, 'Home')).toEqual({ focus: id(work) });
  });

  it('open, enter, close and leave folders with Right and Left', () => {
    const closed = { ...tree, collapsed: new Set([id(work)]) };
    expect(press(work, 'ArrowRight', closed, visibleEntries(entries, closed.collapsed))).toEqual({
      toggle: id(work),
    });
    expect(press(work, 'ArrowRight')).toEqual({ focus: id(clients) });
    expect(press(work, 'ArrowLeft')).toEqual({ toggle: id(work) });
    expect(press(acme, 'ArrowLeft')).toEqual({ focus: id(clients) });
    expect(press(home, 'ArrowLeft')).toBeNull();
    expect(press(admin, 'ArrowRight')).toBeNull();
    expect(press(work, 'a')).toBeNull();
  });
});
