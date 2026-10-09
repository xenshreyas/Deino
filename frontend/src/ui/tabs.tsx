/**
 * Underline tabs (ui.md §3 Tabs): 36px, muted idle, ink and 600 when active,
 * a 2px ink indicator, and a hairline under the whole row.
 */
import * as TabsPrimitive from '@radix-ui/react-tabs';
import { type ComponentPropsWithoutRef, type ElementRef, forwardRef } from 'react';

import { cn } from './cn';

export const Tabs = TabsPrimitive.Root;

/** The tab row with its bottom hairline. */
export const TabsList = forwardRef<
  ElementRef<typeof TabsPrimitive.List>,
  ComponentPropsWithoutRef<typeof TabsPrimitive.List>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.List
    ref={ref}
    className={cn('flex h-9 items-stretch gap-5 border-b border-border', className)}
    {...props}
  />
));
TabsList.displayName = 'TabsList';

/** One tab; the indicator is a pseudo-element so it never shifts layout. */
export const TabsTrigger = forwardRef<
  ElementRef<typeof TabsPrimitive.Trigger>,
  ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    className={cn(
      'relative -mb-px inline-flex items-center gap-1.5 text-sm whitespace-nowrap text-fg-muted',
      'transition-colors duration-(--dur-fast) ease-out hover:text-fg',
      'after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:rounded-full after:bg-fg',
      'after:scale-x-0 after:transition-transform after:duration-(--dur-base) after:ease-out',
      'data-[state=active]:font-semibold data-[state=active]:text-fg data-[state=active]:after:scale-x-100',
      'focus-visible:outline-offset-[-2px] disabled:pointer-events-none disabled:opacity-40',
      className,
    )}
    {...props}
  />
));
TabsTrigger.displayName = 'TabsTrigger';

/** A tab's panel. */
export const TabsContent = forwardRef<
  ElementRef<typeof TabsPrimitive.Content>,
  ComponentPropsWithoutRef<typeof TabsPrimitive.Content>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Content
    ref={ref}
    className={cn('outline-none focus-visible:outline-offset-4', className)}
    {...props}
  />
));
TabsContent.displayName = 'TabsContent';
