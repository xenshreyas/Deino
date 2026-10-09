/**
 * The ⌘J chat bar, as a placeholder until Ask Deino (AI-02) ships: the
 * floating "Ask anything" pill of ui.md §3 Chat bar, read-only, with a
 * one-line note. Escape or ⌘J closes it.
 */
import * as DialogPrimitive from '@radix-ui/react-dialog';

import { VisuallyHidden } from '@/ui/controls';
import { IconButton } from '@/ui/icon-button';
import { ICON_PROPS, MessageCircle, X } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';

import { shellCopy } from '../copy';
import { useShellLayout } from '../layout-store';
import { SHORTCUTS } from '../shortcuts';

/** The chat bar overlay. */
export function ChatPlaceholder() {
  const open = useShellLayout((state) => state.chatOpen);
  const setOpen = useShellLayout((state) => state.setChatOpen);
  return (
    <DialogPrimitive.Root open={open} onOpenChange={setOpen} modal={false}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Content
          aria-describedby="chat-placeholder-note"
          // Stay open while focus moves elsewhere; a click outside or Escape closes it.
          onFocusOutside={(event) => {
            event.preventDefault();
          }}
          className="fixed bottom-4 left-1/2 z-(--z-chatbar) flex w-[min(640px,calc(100%-3rem))] -translate-x-1/2 flex-col gap-2 duration-(--dur-slow) ease-out data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:slide-in-from-bottom-2"
        >
          <VisuallyHidden>
            <DialogPrimitive.Title>{shellCopy.chatLabel}</DialogPrimitive.Title>
          </VisuallyHidden>
          <p id="chat-placeholder-note" className="text-center text-xs text-fg-muted">
            {shellCopy.chatComingSoon}
          </p>
          <div className="flex h-11 items-center gap-2 rounded-full bg-surface pr-1.5 pl-4 shadow-chatbar">
            <MessageCircle {...ICON_PROPS} className="shrink-0 text-fg-subtle" />
            <span className="flex-1 text-sm text-fg-subtle">{shellCopy.chatPlaceholder}</span>
            <Kbd>{SHORTCUTS.chat.display}</Kbd>
            <IconButton
              icon={X}
              label={shellCopy.close}
              tooltipSide="top"
              className="rounded-full"
              onClick={() => {
                setOpen(false);
              }}
            />
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
