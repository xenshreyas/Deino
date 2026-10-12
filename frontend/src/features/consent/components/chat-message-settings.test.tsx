import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { type ChatMessage, ChatMessageCard, messageError } from './chat-message-settings';

const OFF: ChatMessage = { zoom: false, teams: false, meet: false, message: 'Deino is on.' };

function renderCard(chat: ChatMessage, needsPermission = false) {
  const onSave = vi.fn();
  const onOpenSettings = vi.fn();
  render(
    <ChatMessageCard
      chat={chat}
      busy={false}
      needsPermission={needsPermission}
      onSave={onSave}
      onOpenSettings={onOpenSettings}
    />,
  );
  return { onSave, onOpenSettings };
}

describe('ChatMessageCard', () => {
  it('lists every app off by default and turns one on', () => {
    const { onSave } = renderCard(OFF);
    const switches = screen.getAllByRole('switch');

    expect(switches).toHaveLength(3);
    for (const toggle of switches) expect(toggle.getAttribute('aria-checked')).toBe('false');
    fireEvent.click(screen.getByRole('switch', { name: 'Google Meet' }));

    expect(onSave).toHaveBeenCalledWith({ meet: true });
  });

  it('saves the message when the field loses focus, never a blank one while an app is on', () => {
    const { onSave } = renderCard({ ...OFF, zoom: true });
    const field = screen.getByRole('textbox', { name: 'Message' });

    fireEvent.change(field, { target: { value: '   ' } });
    fireEvent.blur(field);
    expect(onSave).not.toHaveBeenCalled();

    fireEvent.change(field, { target: { value: 'Transcribing for my notes.' } });
    fireEvent.blur(field);
    expect(onSave).toHaveBeenCalledWith({ message: 'Transcribing for my notes.' });
  });

  it('offers System Settings only when an app is on and permission is missing', () => {
    renderCard(OFF, true);
    expect(screen.queryByRole('button', { name: 'Open System Settings' })).toBeNull();
  });

  it('opens System Settings from the permission hint', () => {
    const { onOpenSettings } = renderCard({ ...OFF, teams: true }, true);

    fireEvent.click(screen.getByRole('button', { name: 'Open System Settings' }));

    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });
});

describe('messageError', () => {
  it('rejects a blank message only while an app is on', () => {
    expect(messageError(OFF, '')).toBeNull();
    expect(messageError({ ...OFF, meet: true }, ' ')).not.toBeNull();
    expect(messageError({ ...OFF, meet: true }, 'hi')).toBeNull();
  });
});
