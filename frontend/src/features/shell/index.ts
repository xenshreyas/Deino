/**
 * The app shell: providers, sidebar (from the `nav.ts` registry), title bar,
 * ⌘K palette (provider registry), global shortcuts, onboarding
 * gate and drop import.
 */
export { AppShell } from './app-shell';
export { ComingSoonScreen } from './components/coming-soon';
export { TitleBarActions } from './components/title-bar';
export { useShellLayout } from './layout-store';
export { NAV_ITEMS, type NavItem, type NavItemId } from './nav';
export { requestRecordingStart } from './recording';
export { SHORTCUTS } from './shortcuts';
