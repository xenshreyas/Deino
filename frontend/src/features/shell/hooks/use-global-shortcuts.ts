import { useRouter } from '@tanstack/react-router';

import { useMountEffect } from '@/hooks/useMountEffect';

import { useShellLayout } from '../layout-store';
import { requestRecordingStart } from '../recording';
import { findShortcut, isTypingTarget, type ShortcutId } from '../shortcuts';

/** Shortcuts that are plain keys and so must not fire while typing. */
const BARE_KEYS: ReadonlySet<ShortcutId> = new Set(['toggleSidebar', 'toggleRightPanel']);

type Router = ReturnType<typeof useRouter>;

/** Runs the action behind a shortcut. */
function runShortcut(id: ShortcutId, router: Router): void {
  const layout = useShellLayout.getState();
  const goHome = () => void router.navigate({ to: '/' });
  const actions: Record<ShortcutId, () => void> = {
    palette: () => {
      layout.setPaletteOpen(!layout.paletteOpen);
    },
    chat: () => {
      layout.setChatOpen(!layout.chatOpen);
    },
    newNote: goHome,
    record: () => {
      void requestRecordingStart(goHome);
    },
    settings: () => void router.navigate({ to: '/settings' }),
    toggleSidebar: layout.toggleSidebar,
    toggleRightPanel: layout.toggleRightPanel,
  };
  actions[id]();
}

/** Handles one keydown against the shortcut registry. */
function handleKeyDown(event: KeyboardEvent, router: Router): void {
  if (event.defaultPrevented || event.isComposing) return;
  const id = findShortcut(event);
  if (id === undefined) return;
  if (BARE_KEYS.has(id) && isTypingTarget(event.target)) return;
  event.preventDefault();
  runShortcut(id, router);
}

/**
 * Binds the global shortcuts (⌘K, ⌘J, ⌘N, ⌘⇧R, ⌘, and the `[` `]` panel
 * toggles) for the lifetime of the shell.
 */
export function useGlobalShortcuts(): void {
  const router = useRouter();
  useMountEffect(() => {
    const listener = (event: KeyboardEvent) => {
      handleKeyDown(event, router);
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}
