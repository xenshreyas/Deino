import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { HotkeysCard } from './components/hotkey-settings';
import { hotkeysCopy as copy } from './copy';
import type { Hotkey } from './types';

const HOTKEYS: Hotkey[] = [
  {
    action: 'record',
    label: 'Start or stop recording',
    accelerator: 'Ctrl+Alt+Cmd+R',
    defaultAccelerator: 'Ctrl+Alt+Cmd+R',
    state: { kind: 'active' },
  },
  {
    action: 'quick_note',
    label: 'Quick note',
    accelerator: null,
    defaultAccelerator: 'Ctrl+Alt+Cmd+N',
    state: { kind: 'off' },
  },
  {
    action: 'ask',
    label: 'Ask',
    accelerator: 'Ctrl+Alt+Cmd+J',
    defaultAccelerator: 'Ctrl+Alt+Cmd+J',
    state: { kind: 'failed', reason: 'another app is using that shortcut' },
  },
];

function renderCard(refused: { action: 'record'; message: string } | null = null) {
  const onChange = vi.fn();
  render(<HotkeysCard hotkeys={HOTKEYS} saving={false} refused={refused} onChange={onChange} />);
  return onChange;
}

describe('HotkeysCard', () => {
  it('shows each combination as key symbols, or Off, and why one fails', () => {
    renderCard();
    expect(screen.getByText('⌃⌥⌘R')).toBeTruthy();
    expect(screen.getByText(copy.off)).toBeTruthy();
    expect(screen.getByRole('alert').textContent).toBe('another app is using that shortcut');
  });

  it('takes the next combination pressed after Change', () => {
    const onChange = renderCard();
    fireEvent.click(screen.getAllByRole('button', { name: copy.change })[0]!);
    const capture = screen.getByRole('button', { name: copy.listeningHint });
    fireEvent.keyDown(capture, { code: 'MetaLeft', metaKey: true });
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.keyDown(capture, { code: 'KeyK', ctrlKey: true, altKey: true });
    expect(onChange).toHaveBeenCalledWith('record', 'Ctrl+Alt+K');
  });

  it('cancels listening on Escape', () => {
    const onChange = renderCard();
    fireEvent.click(screen.getAllByRole('button', { name: copy.change })[0]!);
    fireEvent.keyDown(screen.getByRole('button', { name: copy.listeningHint }), {
      code: 'Escape',
    });
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: copy.listeningHint })).toBeNull();
  });

  it('turns a hotkey off and resets one to its default', () => {
    const onChange = renderCard();
    fireEvent.click(screen.getAllByRole('button', { name: copy.turnOff })[0]!);
    expect(onChange).toHaveBeenCalledWith('record', null);
    fireEvent.click(screen.getByRole('button', { name: copy.reset }));
    expect(onChange).toHaveBeenCalledWith('quick_note', 'Ctrl+Alt+Cmd+N');
  });

  it('shows a refused change under its row', () => {
    renderCard({ action: 'record', message: 'already used for “Ask”' });
    expect(screen.getAllByRole('alert')[0]?.textContent).toBe('already used for “Ask”');
  });
});
