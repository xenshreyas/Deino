import { describe, expect, it } from 'vitest';

import { acceleratorFromPress, formatAccelerator, type KeyPress } from './accelerator';

const press = (code: string, mods: Partial<KeyPress> = {}): KeyPress => ({
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  code,
  ...mods,
});

describe('acceleratorFromPress', () => {
  it('spells a combination the way Rust stores it', () => {
    const all = { ctrlKey: true, altKey: true, metaKey: true };
    expect(acceleratorFromPress(press('KeyR', all))).toBe('Ctrl+Alt+Cmd+R');
    expect(acceleratorFromPress(press('Digit1', { shiftKey: true, metaKey: true }))).toBe(
      'Shift+Cmd+1',
    );
    expect(acceleratorFromPress(press('F13'))).toBe('F13');
    expect(acceleratorFromPress(press('Space', { ctrlKey: true, altKey: true }))).toBe(
      'Ctrl+Alt+Space',
    );
  });

  it('keeps listening while only modifiers are down or the key is unusable', () => {
    expect(acceleratorFromPress(press('MetaLeft', { metaKey: true }))).toBeNull();
    expect(acceleratorFromPress(press('Escape'))).toBeNull();
    expect(acceleratorFromPress(press('F21'))).toBeNull();
  });
});

describe('formatAccelerator', () => {
  it('shows macOS key symbols in the usual order', () => {
    expect(formatAccelerator('Ctrl+Alt+Cmd+R')).toBe('⌃⌥⌘R');
    expect(formatAccelerator('Shift+Cmd+ArrowUp')).toBe('⇧⌘↑');
    expect(formatAccelerator('Ctrl+Alt+Comma')).toBe('⌃⌥,');
    expect(formatAccelerator('F13')).toBe('F13');
  });
});
