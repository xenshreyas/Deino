/**
 * Select (Radix): the input box with a chevron, and a popover-spec list
 * (ui.md §3 Inputs, Popover).
 */
import * as SelectPrimitive from '@radix-ui/react-select';
import { type ComponentPropsWithoutRef, type ElementRef, forwardRef } from 'react';

import { cn } from './cn';
import { Check, ChevronDown, ICON_PROPS } from './icons';
import { fieldClassName } from './input';
import { floatingMotion, floatingSurface } from './popover';

export const Select = SelectPrimitive.Root;
export const SelectGroup = SelectPrimitive.Group;
export const SelectValue = SelectPrimitive.Value;

/** The closed select: a 32px field with a muted chevron. */
export const SelectTrigger = forwardRef<
  ElementRef<typeof SelectPrimitive.Trigger>,
  ComponentPropsWithoutRef<typeof SelectPrimitive.Trigger>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Trigger
    ref={ref}
    className={cn(
      fieldClassName,
      'flex h-8 items-center justify-between gap-2 px-2.5 text-left whitespace-nowrap data-placeholder:text-fg-subtle [&>span]:line-clamp-1',
      className,
    )}
    {...props}
  >
    {children}
    <SelectPrimitive.Icon asChild>
      <ChevronDown {...ICON_PROPS} className="shrink-0 text-fg-muted" />
    </SelectPrimitive.Icon>
  </SelectPrimitive.Trigger>
));
SelectTrigger.displayName = 'SelectTrigger';

/** The open list, portalled and sized to the trigger. */
export const SelectContent = forwardRef<
  ElementRef<typeof SelectPrimitive.Content>,
  ComponentPropsWithoutRef<typeof SelectPrimitive.Content>
>(({ className, children, position = 'popper', ...props }, ref) => (
  <SelectPrimitive.Portal>
    <SelectPrimitive.Content
      ref={ref}
      position={position}
      sideOffset={4}
      className={cn(
        floatingSurface,
        floatingMotion,
        'relative max-h-[min(360px,var(--radix-select-content-available-height))] min-w-48 overflow-x-hidden overflow-y-auto',
        className,
      )}
      {...props}
    >
      <SelectPrimitive.Viewport
        className={cn(position === 'popper' && 'w-full min-w-(--radix-select-trigger-width)')}
      >
        {children}
      </SelectPrimitive.Viewport>
    </SelectPrimitive.Content>
  </SelectPrimitive.Portal>
));
SelectContent.displayName = 'SelectContent';

/** A group label inside the list, 12/600 muted. */
export const SelectLabel = forwardRef<
  ElementRef<typeof SelectPrimitive.Label>,
  ComponentPropsWithoutRef<typeof SelectPrimitive.Label>
>(({ className, ...props }, ref) => (
  <SelectPrimitive.Label
    ref={ref}
    className={cn('px-2 pt-1.5 pb-0.5 text-xs font-semibold text-fg-muted', className)}
    {...props}
  />
));
SelectLabel.displayName = 'SelectLabel';

/** One option, 30px, with a check when selected. */
export const SelectItem = forwardRef<
  ElementRef<typeof SelectPrimitive.Item>,
  ComponentPropsWithoutRef<typeof SelectPrimitive.Item>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Item
    ref={ref}
    className={cn(
      'relative flex min-h-[30px] w-full cursor-default items-center rounded-sm py-1 pr-8 pl-2 text-sm outline-none select-none',
      'focus:bg-bg-muted data-disabled:pointer-events-none data-disabled:opacity-40',
      className,
    )}
    {...props}
  >
    <span className="absolute right-2 flex items-center text-fg-muted">
      <SelectPrimitive.ItemIndicator>
        <Check {...ICON_PROPS} />
      </SelectPrimitive.ItemIndicator>
    </span>
    <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
  </SelectPrimitive.Item>
));
SelectItem.displayName = 'SelectItem';

/** A hairline between groups. */
export const SelectSeparator = forwardRef<
  ElementRef<typeof SelectPrimitive.Separator>,
  ComponentPropsWithoutRef<typeof SelectPrimitive.Separator>
>(({ className, ...props }, ref) => (
  <SelectPrimitive.Separator
    ref={ref}
    className={cn('-mx-1 my-1 h-px bg-border', className)}
    {...props}
  />
));
SelectSeparator.displayName = 'SelectSeparator';
