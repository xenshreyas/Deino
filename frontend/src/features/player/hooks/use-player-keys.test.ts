import { describe, expect, it, vi } from 'vitest';

import { playerKeyAction } from './use-player-keys';

/** A key event on `target`, with no modifiers unless given. */
function key(name: string, target: EventTarget | null, modifiers: Partial<KeyboardEvent> = {}) {
  return {
    key: name,
    target,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    defaultPrevented: false,
    ...modifiers,
  };
}

function run(event: ReturnType<typeof key>) {
  const actions = { toggle: vi.fn(), skip: vi.fn() };
  playerKeyAction(event, 15_000)?.(actions);
  return actions;
}

describe('playerKeyAction', () => {
  it('plays with Space and moves 15 s with the arrows on the page', () => {
    const line = document.createElement('div');
    line.setAttribute('role', 'listitem');

    expect(run(key(' ', document.body)).toggle).toHaveBeenCalledOnce();
    expect(run(key('ArrowLeft', line)).skip).toHaveBeenCalledWith(-15_000);
    expect(run(key('ArrowRight', line)).skip).toHaveBeenCalledWith(15_000);
  });

  it('leaves fields, buttons, the editor, sliders and modified keys alone', () => {
    const editor = document.createElement('div');
    editor.contentEditable = 'true';
    const slider = document.createElement('div');
    slider.setAttribute('role', 'slider');
    const owners = [
      document.createElement('input'),
      document.createElement('textarea'),
      document.createElement('button'),
      editor,
      slider,
    ];

    for (const owner of owners) expect(playerKeyAction(key(' ', owner), 15_000)).toBeUndefined();
    expect(playerKeyAction(key(' ', document.body, { metaKey: true }), 15_000)).toBeUndefined();
    expect(
      playerKeyAction(key('ArrowLeft', document.body, { defaultPrevented: true }), 15_000),
    ).toBeUndefined();
  });
});
