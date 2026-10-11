import { describe, expect, test } from 'vitest';

import { clampSidebarWidth, createShellLayoutStore, SIDEBAR_WIDTH } from './layout-store';
import { isNavItemActive, NAV_ITEMS, navItem, navItemsIn } from './nav';
import { findShortcut, isTypingTarget, type KeyPress } from './shortcuts';

const press = (key: string, modifiers: Partial<KeyPress> = {}): KeyPress => ({
  key,
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  ...modifiers,
});

describe('shortcuts', () => {
  test('each global shortcut matches its keys exactly', () => {
    expect(findShortcut(press('k', { metaKey: true }))).toBe('palette');
    expect(findShortcut(press('R', { metaKey: true, shiftKey: true }))).toBe('record');
    expect(findShortcut(press('[', {}))).toBe('toggleSidebar');
    expect(findShortcut(press('k', {}))).toBeUndefined();
    expect(findShortcut(press('k', { metaKey: true, altKey: true }))).toBeUndefined();
  });

  test('bare keys stay quiet while the user types in a field or the editor', () => {
    expect(isTypingTarget(document.createElement('input'))).toBe(true);
    const editor = document.createElement('div');
    editor.contentEditable = 'true';
    document.body.append(editor);
    expect(isTypingTarget(editor)).toBe(true);
    expect(isTypingTarget(document.createElement('button'))).toBe(false);
    editor.remove();
  });
});

describe('nav registry', () => {
  test('ids and paths are unique, so every entry is one destination', () => {
    expect(new Set(NAV_ITEMS.map((item) => item.id)).size).toBe(NAV_ITEMS.length);
    expect(new Set(NAV_ITEMS.map((item) => item.to)).size).toBe(NAV_ITEMS.length);
  });

  test('sections list their items in order, with Settings pinned to the footer', () => {
    expect(navItemsIn('primary').map((item) => item.id)).toEqual([
      'home',
      'meetings',
      'action-items',
    ]);
    expect(navItemsIn('footer').map((item) => item.id)).toEqual(['settings']);
  });

  test('Home is active only on its own page; others stay active on child pages', () => {
    expect(isNavItemActive(navItem('home'), '/meetings')).toBe(false);
    expect(isNavItemActive(navItem('meetings'), '/meetings/meeting-1')).toBe(true);
    expect(isNavItemActive(navItem('people'), '/peoplex')).toBe(false);
  });
});

describe('layout store', () => {
  test('the sidebar width stays between 200 and 320px', () => {
    expect(clampSidebarWidth(90)).toBe(SIDEBAR_WIDTH.min);
    expect(clampSidebarWidth(999)).toBe(SIDEBAR_WIDTH.max);
    const store = createShellLayoutStore();
    store.getState().setSidebarWidth(250.4);
    expect(store.getState().sidebarWidth).toBe(250);
  });

  test('the palette and the chat bar never stack', () => {
    const store = createShellLayoutStore();
    store.getState().setPaletteOpen(true);
    store.getState().setChatOpen(true);
    expect(store.getState()).toMatchObject({ chatOpen: true, paletteOpen: false });
  });
});
