/**
 * Popover (Radix): raised surface, r12, popover shadow, 160ms enter from the
 * trigger side (ui.md §3 Popover).
 */
import * as PopoverPrimitive from '@radix-ui/react-popover';
import { type ComponentPropsWithoutRef, type ElementRef, forwardRef } from 'react';

import { cn } from './cn';

export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;
export const PopoverAnchor = PopoverPrimitive.Anchor;

/** The motion every floating layer shares: fade plus a 4px slide from the trigger. */
export const floatingMotion = cn(
  'data-[state=closed]:animate-out data-[state=open]:animate-in',
  'data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0',
  'data-[side=bottom]:slide-in-from-top-1 data-[side=top]:slide-in-from-bottom-1 data-[state=open]:zoom-in-[0.98]',
  'data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1',
  'duration-(--dur-base) ease-out',
);

/** Classes of every floating surface (popover, menu, select list). */
export const floatingSurface =
  'z-(--z-popover) rounded-lg bg-surface-raised p-1 text-sm text-fg shadow-popover outline-none';

/** The popover panel, portalled so it escapes clipped containers. */
export const PopoverContent = forwardRef<
  ElementRef<typeof PopoverPrimitive.Content>,
  ComponentPropsWithoutRef<typeof PopoverPrimitive.Content>
>(({ className, align = 'center', sideOffset = 6, ...props }, ref) => (
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      ref={ref}
      align={align}
      sideOffset={sideOffset}
      className={cn(floatingSurface, floatingMotion, 'w-72 p-3', className)}
      {...props}
    />
  </PopoverPrimitive.Portal>
));
PopoverContent.displayName = 'PopoverContent';
