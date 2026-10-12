/**
 * `deino://` links and hotkeys in the window (DEV-05). Rust parses and
 * validates each link and holds what it shows until Deino is unlocked;
 * {@link useLinkHost} takes those navigations and follows them.
 *
 * | Export | Where |
 * |---|---|
 * | {@link useLinkHost} | the shell's main layout, inside the app lock's gate |
 */
export { linksCopy } from './copy';
export { useLinkHost, type LinkHostOptions } from './hooks/use-link-host';
