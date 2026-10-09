/**
 * Button (ui.md §3 Buttons). Primary is ink, not accent; the accent variant is
 * reserved for rare calls to action such as onboarding.
 */
import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import { type ButtonHTMLAttributes, forwardRef } from 'react';

import { cn } from './cn';

/** Class recipe for a button; also used to style links as buttons. */
export const buttonVariants = cva(
  [
    'inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md whitespace-nowrap select-none',
    'transition-[background-color,color,opacity,transform] duration-(--dur-fast) ease-out',
    'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
    'disabled:pointer-events-none disabled:opacity-40',
    "[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  ],
  {
    variants: {
      variant: {
        primary: 'bg-fg text-on-fg hover:opacity-90 active:translate-y-px active:opacity-80',
        secondary:
          'border border-control-border bg-control text-fg shadow-xs hover:bg-bg-subtle active:bg-bg-muted',
        ghost: 'text-fg-muted hover:bg-bg-muted hover:text-fg',
        accent: 'bg-accent text-accent-fg hover:opacity-90 active:translate-y-px',
        danger: 'text-danger hover:bg-danger-soft',
        'danger-solid': 'bg-danger text-on-fg hover:opacity-90 active:translate-y-px',
        link: 'h-auto px-0 text-accent underline-offset-4 hover:underline',
      },
      size: {
        sm: 'h-7 px-2.5 text-sm',
        md: 'h-8 px-3 text-sm',
        lg: 'h-10 px-4 text-sm font-semibold',
        icon: 'size-7 rounded-sm',
      },
    },
    defaultVariants: { variant: 'primary', size: 'md' },
  },
);

/** Props of {@link Button}. */
export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {
  /** Render the child element (for example a router `Link`) with button styling. */
  asChild?: boolean;
}

/** The house button: `primary`, `secondary`, `ghost`, `accent`, `danger` or `link`. */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Component = asChild ? Slot : 'button';
    return (
      <Component
        ref={ref}
        className={cn(buttonVariants({ variant, size }), className)}
        {...props}
      />
    );
  },
);
Button.displayName = 'Button';
