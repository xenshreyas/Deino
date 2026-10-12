/**
 * Person cards on @mentions (Granola's mention hovers). The notepad's chips
 * are plain DOM that TipTap draws (`<span data-type="mention" data-id>`),
 * so the zone listens once, around the whole editor, for the pointer
 * resting on a chip, and opens the person's card anchored to it. The card
 * stays while the pointer moves onto it, and never takes focus from the
 * editor.
 */
import { type MouseEvent, type ReactNode, useRef, useState } from 'react';

import { HOVER_CLOSE_DELAY, HOVER_OPEN_DELAY } from '@/ui/hover-card';
import { Popover, PopoverAnchor, PopoverContent } from '@/ui/popover';

import { asPersonId, type PersonId } from '../types';
import { PersonCardContent } from './person-hover-card';

/** The chip under the pointer, if any. */
function mentionAt(target: EventTarget): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  const chip = target.closest('[data-type="mention"][data-id]');
  return chip instanceof HTMLElement && chip.dataset.id !== '' ? chip : null;
}

/** The open card and the timers that open and close it. */
function useMentionCard() {
  const [card, setCard] = useState<{ personId: PersonId; chip: HTMLElement } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const later = (delay: number, action: () => void) => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = setTimeout(action, delay);
  };
  const cancel = () => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  };
  const over = (event: MouseEvent) => {
    const chip = mentionAt(event.target);
    if (chip === null) return;
    later(HOVER_OPEN_DELAY, () => {
      setCard({ personId: asPersonId(chip.dataset.id ?? ''), chip });
    });
  };
  const out = (event: MouseEvent) => {
    if (mentionAt(event.target) !== null)
      later(HOVER_CLOSE_DELAY, () => {
        setCard(null);
      });
  };
  return {
    card,
    over,
    out,
    cancel,
    close: () => {
      setCard(null);
    },
    later,
  };
}

/** `children` (the editor) with person cards on its @mentions. */
export function MentionHoverZone({ children }: { children: ReactNode }) {
  const { card, over, out, cancel, close, later } = useMentionCard();
  return (
    <div onMouseOver={over} onMouseOut={out} className="contents">
      {children}
      <Popover
        open={card !== null}
        onOpenChange={(open) => {
          if (!open) close();
        }}
      >
        {card === null ? null : <PopoverAnchor virtualRef={{ current: card.chip }} />}
        <PopoverContent
          align="start"
          onOpenAutoFocus={(event) => {
            event.preventDefault();
          }}
          onMouseEnter={cancel}
          onMouseLeave={() => {
            later(HOVER_CLOSE_DELAY, close);
          }}
        >
          {card === null ? null : <PersonCardContent personId={card.personId} />}
        </PopoverContent>
      </Popover>
    </div>
  );
}
