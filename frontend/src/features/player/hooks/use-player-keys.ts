/**
 * The player's keys on the meeting page: Space plays and pauses, ← and →
 * move 15 s. Keys typed into a field, the notes editor, a menu, a slider
 * (the scrubber has its own steps) or a button (Space presses it) are left
 * alone, as are keys with a modifier.
 */
import { useMountEffect } from '@/hooks/useMountEffect';

/** Roles whose own keyboard handling must win. */
const KEYED_ROLES = new Set([
  'slider',
  'menu',
  'menuitem',
  'menuitemradio',
  'option',
  'listbox',
  'tab',
  'combobox',
  'textbox',
  'searchbox',
  'spinbutton',
]);

/** Elements whose own keyboard handling must win. */
const KEYED_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON', 'A']);

/** Whether `target` handles Space or the arrows itself. */
export function ownsKeys(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || KEYED_TAGS.has(target.tagName)) return true;
  const role = target.getAttribute('role');
  return role !== null && KEYED_ROLES.has(role);
}

/** What the keys do. */
export interface PlayerKeyActions {
  toggle: () => void;
  skip: (deltaMs: number) => void;
}

/** The action for a key event, if it is one of the player's. */
export function playerKeyAction(
  event: Pick<
    KeyboardEvent,
    'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'target' | 'defaultPrevented'
  >,
  skipMs: number,
): ((actions: PlayerKeyActions) => void) | undefined {
  if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return undefined;
  if (event.shiftKey || ownsKeys(event.target)) return undefined;
  if (event.key === ' ') return (actions) => actions.toggle();
  if (event.key === 'ArrowLeft') return (actions) => actions.skip(-skipMs);
  if (event.key === 'ArrowRight') return (actions) => actions.skip(skipMs);
  return undefined;
}

/** Listens for the player's keys while the dock is mounted. */
export function usePlayerKeys(actions: PlayerKeyActions, skipMs: number): void {
  useMountEffect(() => {
    const listener = (event: KeyboardEvent) => {
      const action = playerKeyAction(event, skipMs);
      if (action === undefined) return;
      event.preventDefault();
      action(actions);
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}
