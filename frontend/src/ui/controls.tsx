/**
 * Small form controls on the tokens: label, switch, separator, progress and a
 * visually hidden span (ui.md §3 Inputs).
 */
import * as LabelPrimitive from '@radix-ui/react-label';
import * as ProgressPrimitive from '@radix-ui/react-progress';
import * as SeparatorPrimitive from '@radix-ui/react-separator';
import * as SwitchPrimitive from '@radix-ui/react-switch';
import {
  type ComponentPropsWithoutRef,
  type ElementRef,
  forwardRef,
  type HTMLAttributes,
} from 'react';

import { cn } from './cn';

/** A form label, 14px ink. */
export const Label = forwardRef<
  ElementRef<typeof LabelPrimitive.Root>,
  ComponentPropsWithoutRef<typeof LabelPrimitive.Root>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root
    ref={ref}
    className={cn(
      'text-sm text-fg peer-disabled:cursor-not-allowed peer-disabled:opacity-60',
      className,
    )}
    {...props}
  />
));
Label.displayName = 'Label';

/** On/off switch: 32×18 track, accent when on, 120ms. */
export const Switch = forwardRef<
  ElementRef<typeof SwitchPrimitive.Root>,
  ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitive.Root
    ref={ref}
    className={cn(
      'peer inline-flex h-[18px] w-8 shrink-0 items-center rounded-full p-0.5',
      'transition-colors duration-(--dur-fast) ease-out disabled:cursor-not-allowed disabled:opacity-40',
      'data-[state=checked]:bg-accent data-[state=unchecked]:bg-border-strong',
      className,
    )}
    {...props}
  >
    <SwitchPrimitive.Thumb className="pointer-events-none block size-3.5 rounded-full bg-switch-thumb shadow-xs transition-transform duration-(--dur-fast) ease-out data-[state=checked]:translate-x-3.5" />
  </SwitchPrimitive.Root>
));
Switch.displayName = 'Switch';

/** A 1px hairline. */
export const Separator = forwardRef<
  ElementRef<typeof SeparatorPrimitive.Root>,
  ComponentPropsWithoutRef<typeof SeparatorPrimitive.Root>
>(({ className, orientation = 'horizontal', decorative = true, ...props }, ref) => (
  <SeparatorPrimitive.Root
    ref={ref}
    decorative={decorative}
    orientation={orientation}
    className={cn(
      'shrink-0 bg-border',
      orientation === 'horizontal' ? 'h-px w-full' : 'h-full w-px',
      className,
    )}
    {...props}
  />
));
Separator.displayName = 'Separator';

/** A thin accent progress bar (model downloads, imports). */
export const Progress = forwardRef<
  ElementRef<typeof ProgressPrimitive.Root>,
  ComponentPropsWithoutRef<typeof ProgressPrimitive.Root>
>(({ className, value, ...props }, ref) => (
  <ProgressPrimitive.Root
    ref={ref}
    value={value}
    className={cn('relative h-1.5 w-full overflow-hidden rounded-full bg-bg-muted', className)}
    {...props}
  >
    <ProgressPrimitive.Indicator
      className="h-full w-full bg-accent transition-transform duration-(--dur-base) ease-out"
      style={{ transform: `translateX(-${100 - (value ?? 0)}%)` }}
    />
  </ProgressPrimitive.Root>
));
Progress.displayName = 'Progress';

/** Text for screen readers only. */
export const VisuallyHidden = forwardRef<HTMLSpanElement, HTMLAttributes<HTMLSpanElement>>(
  ({ className, ...props }, ref) => (
    <span ref={ref} className={cn('sr-only', className)} {...props} />
  ),
);
VisuallyHidden.displayName = 'VisuallyHidden';
