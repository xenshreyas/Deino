/**
 * The app's global keyboard shortcuts (ui.md §1 rule 7), in one registry so
 * tooltips, menus and the palette show the same keys the handler matches.
 */

/** One shortcut: a key plus modifiers, and how to show it. */
export interface Shortcut {
  /** `KeyboardEvent.key`, lower-case for letters. */
  key: string;
  /** ⌘ on macOS (Ctrl elsewhere). */
  mod?: boolean;
  shift?: boolean;
  /** How the shortcut is shown in a `Kbd`. */
  display: string;
}

/** The names of the global shortcuts. */
const SHORTCUT_IDS = [
  'palette',
  'chat',
  'newNote',
  'record',
  'settings',
  'toggleSidebar',
  'toggleRightPanel',
] as const;

/** The name of a global shortcut. */
export type ShortcutId = (typeof SHORTCUT_IDS)[number];

/** Every global shortcut. */
export const SHORTCUTS: Readonly<Record<ShortcutId, Shortcut>> = {
  palette: { key: 'k', mod: true, display: '⌘K' },
  chat: { key: 'j', mod: true, display: '⌘J' },
  newNote: { key: 'n', mod: true, display: '⌘N' },
  record: { key: 'r', mod: true, shift: true, display: '⌘⇧R' },
  settings: { key: ',', mod: true, display: '⌘,' },
  toggleSidebar: { key: '[', display: '[' },
  toggleRightPanel: { key: ']', display: ']' },
};

/** The parts of a `KeyboardEvent` a match reads (keeps tests DOM-free). */
export interface KeyPress {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/** True when `press` is exactly `shortcut` (no extra modifiers). */
export function matchesShortcut(press: KeyPress, shortcut: Shortcut): boolean {
  const mod = press.metaKey || press.ctrlKey;
  return (
    press.key.toLowerCase() === shortcut.key &&
    mod === (shortcut.mod ?? false) &&
    press.shiftKey === (shortcut.shift ?? false) &&
    !press.altKey
  );
}

/**
 * Shortcuts whose feature has not shipped (UX-27): they keep their keys
 * here, but pressing them does nothing until the feature lands. ⌘J is Ask
 * Deino (AI-02, deprioritised by D13).
 */
const UNSHIPPED: ReadonlySet<ShortcutId> = new Set(['chat']);

/** The shortcut `press` triggers, if any. */
export function findShortcut(press: KeyPress): ShortcutId | undefined {
  return SHORTCUT_IDS.find((id) => !UNSHIPPED.has(id) && matchesShortcut(press, SHORTCUTS[id]));
}

/**
 * True while the user is typing: bare-key shortcuts (`[`, `]`) must not fire
 * inside inputs, text areas or the note editor.
 */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName);
}
