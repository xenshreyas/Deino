/**
 * Hover card (Radix): a raised preview that opens on hover or focus after a
 * short delay and stays while the pointer is on it (ui.md §2.8, the popover
 * layer). For previews only; anything the user must act on lives in a
 * popover or a page.
 */
import { HoverCard as HoverCardPrimitive } from 'radix-ui';
import { type ComponentPropsWithoutRef, type ElementRef, forwardRef } from 'react';

import { cn } from './cn';
import { floatingMotion, floatingSurface } from './popover';

export const HoverCard = HoverCardPrimitive.Root;
export const HoverCardTrigger = HoverCardPrimitive.Trigger;

/** How long the pointer rests before a card opens, and lingers after, in ms. */
export const HOVER_OPEN_DELAY = 350;
export const HOVER_CLOSE_DELAY = 120;

/** The card, portalled so it escapes clipped containers. */
export const HoverCardContent = forwardRef<
  ElementRef<typeof HoverCardPrimitive.Content>,
  ComponentPropsWithoutRef<typeof HoverCardPrimitive.Content>
>(({ className, align = 'start', sideOffset = 6, ...props }, ref) => (
  <HoverCardPrimitive.Portal>
    <HoverCardPrimitive.Content
      ref={ref}
      align={align}
      sideOffset={sideOffset}
      className={cn(floatingSurface, floatingMotion, 'w-72 p-3', className)}
      {...props}
    />
  </HoverCardPrimitive.Portal>
));
HoverCardContent.displayName = 'HoverCardContent';
