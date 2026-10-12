import { describe, expect, it } from 'vitest';

import { TRANSCRIPT_ACTIONS } from '../actions';
import { layoutToolbar, toolbarSlots } from './toolbar';

const byId = (ids: string[]) => TRANSCRIPT_ACTIONS.filter((action) => ids.includes(action.id));
const ids = (actions: readonly { id: string }[]) => actions.map((action) => action.id);

describe('toolbarSlots', () => {
  it('fits more actions beside bubbles in a wider column, never fewer than two', () => {
    expect(toolbarSlots(720)).toBe(4);
    expect(toolbarSlots(380)).toBe(2);
    expect(toolbarSlots(0)).toBe(2);
  });
});

describe('layoutToolbar', () => {
  it('shows every action when they fit', () => {
    const shown = byId(['play', 'copy', 'edit', 'delete']);

    expect(ids(layoutToolbar(shown, 4).inline)).toEqual(['play', 'copy', 'edit', 'delete']);
  });

  it('folds the rare ones into ⋯ first, keeping toolbar order', () => {
    const shown = byId(['play', 'copy', 'edit', 'delete']);

    const layout = layoutToolbar(shown, 3);

    expect(ids(layout.inline)).toEqual(['play', 'copy']);
    expect(ids(layout.overflow)).toEqual(['edit', 'delete']);
  });

  it('keeps one action and ⋯ in the narrowest toolbar', () => {
    const layout = layoutToolbar(byId(['copy', 'edit', 'delete']), 2);

    expect(ids(layout.inline)).toEqual(['copy']);
    expect(ids(layout.overflow)).toEqual(['edit', 'delete']);
  });
});
