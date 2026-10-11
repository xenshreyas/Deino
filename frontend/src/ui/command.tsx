/**
 * cmdk primitives on the tokens (ui.md §3 Command palette): 48px input with a
 * search icon, 12/600 group labels, 36px items, `bg-muted` selection. Used by
 * the ⌘K palette and by comboboxes inside popovers.
 */
import { Command as CommandPrimitive } from 'cmdk';
import {
  type ComponentPropsWithoutRef,
  type ElementRef,
  forwardRef,
  type HTMLAttributes,
  type ReactNode,
} from 'react';

import { cn } from './cn';
import { ICON_PROPS, Search } from './icons';

/** The cmdk root. */
export const Command = forwardRef<
  ElementRef<typeof CommandPrimitive>,
  ComponentPropsWithoutRef<typeof CommandPrimitive>
>(({ className, ...props }, ref) => (
  <CommandPrimitive
    ref={ref}
    className={cn('flex h-full w-full flex-col overflow-hidden text-fg', className)}
    {...props}
  />
));
Command.displayName = 'Command';

/** Props of {@link CommandInput}. */
export interface CommandInputProps extends ComponentPropsWithoutRef<typeof CommandPrimitive.Input> {
  /** `palette` is the ⌘K palette's 48px input; `combobox` fits a popover. */
  variant?: 'combobox' | 'palette';
  /** Shown between the icon and the text, such as the palette's filter chips. */
  before?: ReactNode;
}

/** The search field with its icon and a hairline underneath. */
export const CommandInput = forwardRef<
  ElementRef<typeof CommandPrimitive.Input>,
  CommandInputProps
>(({ className, variant = 'combobox', before, ...props }, ref) => (
  <div
    className={cn(
      'flex items-center gap-2.5 border-b border-border',
      variant === 'palette' ? 'px-4' : 'px-3',
    )}
  >
    <Search
      {...ICON_PROPS}
      size={variant === 'palette' ? 20 : 16}
      className="shrink-0 text-fg-muted"
    />
    {before}
    <CommandPrimitive.Input
      ref={ref}
      className={cn(
        'w-full bg-transparent text-fg outline-none placeholder:text-fg-subtle disabled:opacity-40',
        variant === 'palette' ? 'h-12 text-base' : 'h-9 text-sm',
        className,
      )}
      {...props}
    />
  </div>
));
CommandInput.displayName = 'CommandInput';

/** The scrollable result list. */
export const CommandList = forwardRef<
  ElementRef<typeof CommandPrimitive.List>,
  ComponentPropsWithoutRef<typeof CommandPrimitive.List>
>(({ className, ...props }, ref) => (
  <CommandPrimitive.List
    ref={ref}
    className={cn('max-h-[400px] overflow-x-hidden overflow-y-auto p-1.5', className)}
    {...props}
  />
));
CommandList.displayName = 'CommandList';

/** Shown when nothing matches. */
export const CommandEmpty = forwardRef<
  ElementRef<typeof CommandPrimitive.Empty>,
  ComponentPropsWithoutRef<typeof CommandPrimitive.Empty>
>(({ className, ...props }, ref) => (
  <CommandPrimitive.Empty
    ref={ref}
    className={cn('py-8 text-center text-sm text-fg-muted', className)}
    {...props}
  />
));
CommandEmpty.displayName = 'CommandEmpty';

/** A labelled group of results. */
export const CommandGroup = forwardRef<
  ElementRef<typeof CommandPrimitive.Group>,
  ComponentPropsWithoutRef<typeof CommandPrimitive.Group>
>(({ className, ...props }, ref) => (
  <CommandPrimitive.Group
    ref={ref}
    className={cn(
      '**:[[cmdk-group-heading]]:px-2.5 **:[[cmdk-group-heading]]:pt-2 **:[[cmdk-group-heading]]:pb-1',
      '**:[[cmdk-group-heading]]:text-xs **:[[cmdk-group-heading]]:font-semibold **:[[cmdk-group-heading]]:text-fg-muted',
      className,
    )}
    {...props}
  />
));
CommandGroup.displayName = 'CommandGroup';

/** One result: 36px, icon, label, trailing meta or `Kbd`. */
export const CommandItem = forwardRef<
  ElementRef<typeof CommandPrimitive.Item>,
  ComponentPropsWithoutRef<typeof CommandPrimitive.Item>
>(({ className, ...props }, ref) => (
  <CommandPrimitive.Item
    ref={ref}
    className={cn(
      'relative flex min-h-9 cursor-default items-center gap-2.5 rounded-sm px-2.5 text-sm outline-none select-none',
      'data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-40 data-[selected=true]:bg-bg-muted',
      '[&>svg]:shrink-0 [&>svg]:text-fg-muted',
      className,
    )}
    {...props}
  />
));
CommandItem.displayName = 'CommandItem';

/** A hairline between groups. */
export const CommandSeparator = forwardRef<
  ElementRef<typeof CommandPrimitive.Separator>,
  ComponentPropsWithoutRef<typeof CommandPrimitive.Separator>
>(({ className, ...props }, ref) => (
  <CommandPrimitive.Separator
    ref={ref}
    className={cn('-mx-1.5 my-1 h-px bg-border', className)}
    {...props}
  />
));
CommandSeparator.displayName = 'CommandSeparator';

/** Trailing meta text of an item. */
export function CommandShortcut({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn('ml-auto text-xs text-fg-subtle', className)} {...props} />;
}
