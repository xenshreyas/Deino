/**
 * Tooltip (Radix), restyled to the tokens. Icon-only buttons always get one,
 * with the shortcut in a {@link Kbd} (ui.md §1 rule 7).
 */
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { type ComponentPropsWithoutRef, type ElementRef, forwardRef, type ReactNode } from 'react';

import { cn } from './cn';
import { Kbd } from './kbd';

/** Shares one open delay across every tooltip in the app. */
export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

/** The floating label: raised surface, 12px text, popover shadow. */
export const TooltipContent = forwardRef<
  ElementRef<typeof TooltipPrimitive.Content>,
  ComponentPropsWithoutRef<typeof TooltipPrimitive.Content>
>(({ className, sideOffset = 6, ...props }, ref) => (
  <TooltipPrimitive.Portal>
    <TooltipPrimitive.Content
      ref={ref}
      sideOffset={sideOffset}
      className={cn(
        'z-(--z-tooltip) flex items-center gap-2 rounded-md bg-surface-raised px-2 py-1 text-xs text-fg shadow-popover',
        'origin-(--radix-tooltip-content-transform-origin) animate-in duration-(--dur-fast) fade-in-0',
        className,
      )}
      {...props}
    />
  </TooltipPrimitive.Portal>
));
TooltipContent.displayName = 'TooltipContent';

/** Props of {@link Hint}. */
export interface HintProps {
  /** What the control does, for example "New note". */
  label: string;
  /** The shortcut, in display form (`⌘N`). */
  shortcut?: string | undefined;
  side?: 'top' | 'right' | 'bottom' | 'left' | undefined;
  /** The trigger; it must accept a ref (a button). */
  children: ReactNode;
}

/** A tooltip with a label and an optional shortcut around one control. */
export function Hint({ label, shortcut, side = 'bottom', children }: HintProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side}>
        <span>{label}</span>
        {shortcut === undefined ? null : <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  );
}
