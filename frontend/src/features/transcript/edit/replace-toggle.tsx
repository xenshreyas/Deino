/**
 * Opening the replace row (TRN-02): a toggle beside the search field, and
 * ⌥⌘F while the transcript is on screen. A live transcript has no replace
 * row: lines still being spoken cannot be edited.
 */
import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';
import { Replace } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { transcriptCopy } from '../copy';

/** Whether the replace row is open, and how to toggle it. */
export interface ReplaceToggle {
  open: boolean;
  toggle: () => void;
}

/** The row's state; ⌥⌘F toggles it. Never open while `live`. */
export function useReplaceToggle(live: boolean): ReplaceToggle {
  const [open, setOpen] = useState(false);
  const toggle = () => {
    setOpen((shown) => !shown);
  };
  useMountEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || !event.altKey || event.code !== 'KeyF') return;
      event.preventDefault();
      toggle();
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
  return { open: open && !live, toggle };
}

/** The toggle, pressed while the row is open. */
export function ReplaceToggleButton({ open, toggle }: ReplaceToggle) {
  return (
    <IconButton
      icon={Replace}
      label={transcriptCopy.replaceToggle}
      shortcut={transcriptCopy.replaceShortcut}
      aria-pressed={open}
      className={open ? 'bg-bg-muted text-fg' : undefined}
      onClick={toggle}
    />
  );
}
