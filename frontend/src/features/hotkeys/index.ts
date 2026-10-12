/**
 * System-wide hotkeys (DEV-05): Settings → Shortcuts. Rust registers them
 * with macOS and runs them while Deino is in the background; this feature
 * shows and changes them.
 *
 * | Export | Where |
 * |---|---|
 * | {@link HotkeySettings} | Settings → Shortcuts (`./settings-page.ts`) |
 */
export { formatAccelerator } from './accelerator';
export { HotkeySettings } from './components/hotkey-settings';
export { hotkeysCopy } from './copy';
