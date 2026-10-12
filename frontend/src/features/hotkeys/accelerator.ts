/**
 * Hotkey combinations in the text Rust stores (`Ctrl+Alt+Cmd+R`, the
 * spelling `tauri-plugin-global-shortcut` reads): built from a key press,
 * and shown as macOS key symbols (`⌃⌥⌘R`). Rust validates every
 * combination; this only reads keys.
 */

/** A keydown's modifier flags and physical key. */
export interface KeyPress {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  /** `KeyboardEvent.code`, for example `KeyR` or `F5`. */
  code: string;
}

/** Modifiers in Rust's order, with their symbols. */
const MODIFIERS = [
  { flag: 'ctrlKey', name: 'Ctrl', symbol: '⌃' },
  { flag: 'altKey', name: 'Alt', symbol: '⌥' },
  { flag: 'shiftKey', name: 'Shift', symbol: '⇧' },
  { flag: 'metaKey', name: 'Cmd', symbol: '⌘' },
] as const;

/** Named keys a hotkey may use, with how they are shown. */
const NAMED_KEYS: Readonly<Record<string, string>> = {
  Space: 'Space',
  Enter: '↩',
  Tab: '⇥',
  Comma: ',',
  Period: '.',
  Slash: '/',
  Semicolon: ';',
  Quote: "'",
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  Minus: '-',
  Equal: '=',
  Backquote: '`',
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
};

/** The key's name in Rust's spelling, or `null` for a modifier or an unusable key. */
function keyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code)?.[1];
  if (letter !== undefined) return letter;
  const digit = /^Digit(\d)$/.exec(code)?.[1];
  if (digit !== undefined) return digit;
  if (/^F([1-9]|1\d|20)$/.test(code)) return code;
  return code in NAMED_KEYS ? code : null;
}

/**
 * The combination a key press makes, or `null` while only modifiers are
 * down (or the key cannot be used): keep listening.
 */
export function acceleratorFromPress(press: KeyPress): string | null {
  const key = keyName(press.code);
  if (key === null) return null;
  const modifiers = MODIFIERS.filter(({ flag }) => press[flag]).map(({ name }) => name);
  return [...modifiers, key].join('+');
}

/** `Ctrl+Alt+Cmd+R` as `⌃⌥⌘R`; text it cannot read is shown as it is. */
export function formatAccelerator(accelerator: string): string {
  const parts = accelerator.split('+');
  const key = parts.at(-1) ?? '';
  const modifiers = MODIFIERS.filter(({ name }) => parts.slice(0, -1).includes(name));
  return modifiers.map(({ symbol }) => symbol).join('') + (NAMED_KEYS[key] ?? key);
}
