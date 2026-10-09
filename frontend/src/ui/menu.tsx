/**
 * Dropdown menu (Radix) on the popover spec: 30px items, muted icons,
 * trailing shortcut, destructive items in `--danger` (ui.md §3 Popover).
 */
import * as MenuPrimitive from '@radix-ui/react-dropdown-menu';
import {
  type ComponentPropsWithoutRef,
  type ElementRef,
  forwardRef,
  type HTMLAttributes,
} from 'react';

import { cn } from './cn';
import { floatingMotion, floatingSurface } from './popover';

export const DropdownMenu = MenuPrimitive.Root;
export const DropdownMenuTrigger = MenuPrimitive.Trigger;
export const DropdownMenuGroup = MenuPrimitive.Group;

/** The open menu, portalled, at least 200px wide. */
export const DropdownMenuContent = forwardRef<
  ElementRef<typeof MenuPrimitive.Content>,
  ComponentPropsWithoutRef<typeof MenuPrimitive.Content>
>(({ className, sideOffset = 6, ...props }, ref) => (
  <MenuPrimitive.Portal>
    <MenuPrimitive.Content
      ref={ref}
      sideOffset={sideOffset}
      className={cn(
        floatingSurface,
        floatingMotion,
        'max-h-[360px] min-w-[200px] overflow-y-auto',
        className,
      )}
      {...props}
    />
  </MenuPrimitive.Portal>
));
DropdownMenuContent.displayName = 'DropdownMenuContent';

/** Props of {@link DropdownMenuItem}. */
export interface DropdownMenuItemProps extends ComponentPropsWithoutRef<typeof MenuPrimitive.Item> {
  /** Shows the item in `--danger` (delete, remove). */
  destructive?: boolean;
}

/** One action; put a 16px icon first and a `DropdownMenuShortcut` last. */
export const DropdownMenuItem = forwardRef<
  ElementRef<typeof MenuPrimitive.Item>,
  DropdownMenuItemProps
>(({ className, destructive = false, ...props }, ref) => (
  <MenuPrimitive.Item
    ref={ref}
    className={cn(
      'relative flex h-[30px] cursor-default items-center gap-2 rounded-sm px-2 text-sm outline-none select-none',
      'transition-colors duration-(--dur-fast) focus:bg-bg-muted data-disabled:pointer-events-none data-disabled:opacity-40',
      destructive ? 'text-danger [&>svg]:text-danger' : 'text-fg [&>svg]:text-fg-muted',
      className,
    )}
    {...props}
  />
));
DropdownMenuItem.displayName = 'DropdownMenuItem';

/** A group label, 12/600 muted. */
export const DropdownMenuLabel = forwardRef<
  ElementRef<typeof MenuPrimitive.Label>,
  ComponentPropsWithoutRef<typeof MenuPrimitive.Label>
>(({ className, ...props }, ref) => (
  <MenuPrimitive.Label
    ref={ref}
    className={cn('px-2 pt-1.5 pb-0.5 text-xs font-semibold text-fg-muted', className)}
    {...props}
  />
));
DropdownMenuLabel.displayName = 'DropdownMenuLabel';

/** A hairline between groups. */
export const DropdownMenuSeparator = forwardRef<
  ElementRef<typeof MenuPrimitive.Separator>,
  ComponentPropsWithoutRef<typeof MenuPrimitive.Separator>
>(({ className, ...props }, ref) => (
  <MenuPrimitive.Separator
    ref={ref}
    className={cn('-mx-1 my-1 h-px bg-border', className)}
    {...props}
  />
));
DropdownMenuSeparator.displayName = 'DropdownMenuSeparator';

/** The trailing shortcut or meta of an item. */
export function DropdownMenuShortcut({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn('ml-auto text-xs text-fg-subtle', className)} {...props} />;
}
