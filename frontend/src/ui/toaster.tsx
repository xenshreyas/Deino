/**
 * Toasts (sonner), restyled to the tokens (ui.md §3 Toast): bottom-right,
 * 360px, raised surface, r12, popover shadow, a 16px status icon in its
 * semantic colour, at most three visible. `richColors` stays off so success
 * and error never flood the card with colour.
 *
 * **When to toast (UX-32).** A toast confirms only what the user cannot see.
 * Toast for:
 * - a copy to the clipboard;
 * - background work that finishes off-screen (a file saved, an export, an
 *   automation started, a model update the user did not ask for);
 * - a destructive action that can be undone (with an Undo action);
 * - a failure that has nowhere inline to go, and an ask that needs an answer
 *   while the user is elsewhere (a permission at the start of a call).
 *
 * Never toast:
 * - a setting saved: switches and fields save silently and show an inline
 *   error if the save fails;
 * - a change the user can see on screen (a tag, folder, person or template
 *   created, renamed, moved or deleted; a list updated);
 * - a recovery the recording pill already showed, or a download whose
 *   progress was on screen;
 * - anything unfinished ("coming soon").
 */
import { Toaster as Sonner } from 'sonner';

import { CircleAlert, CircleCheck, ICON_PROPS, Info, LoaderCircle, TriangleAlert } from './icons';

const TOAST_CLASSES = {
  toast:
    'group flex w-[360px] items-start gap-2.5 rounded-lg bg-surface-raised px-3.5 py-3 text-sm text-fg shadow-popover font-sans',
  title: 'font-semibold text-fg',
  description: 'text-fg-muted',
  icon: 'mt-0.5 shrink-0',
  actionButton:
    'ml-auto h-7 shrink-0 rounded-md px-2.5 text-sm text-fg-muted transition-colors hover:bg-bg-muted hover:text-fg',
  cancelButton:
    'h-7 shrink-0 rounded-md px-2.5 text-sm text-fg-muted transition-colors hover:bg-bg-muted hover:text-fg',
  closeButton:
    'border-0 bg-surface-raised text-fg-muted shadow-popover hover:bg-bg-muted hover:text-fg',
} as const;

const TOAST_ICONS = {
  success: <CircleCheck {...ICON_PROPS} className="text-success" />,
  error: <CircleAlert {...ICON_PROPS} className="text-danger" />,
  warning: <TriangleAlert {...ICON_PROPS} className="text-warning" />,
  info: <Info {...ICON_PROPS} className="text-fg-muted" />,
  loading: <LoaderCircle {...ICON_PROPS} className="animate-spin text-fg-muted" />,
};

/** Mount once at the app root; call `toast(...)` from `sonner` anywhere. */
export function Toaster() {
  return (
    <Sonner
      position="bottom-right"
      offset={16}
      visibleToasts={3}
      gap={8}
      closeButton
      icons={TOAST_ICONS}
      toastOptions={{ unstyled: true, classNames: TOAST_CLASSES }}
    />
  );
}
