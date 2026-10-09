/**
 * Inline alert: errors and warnings sit next to what they are about, in plain
 * English (ui.md §1 rule 9). Soft tinted fill, no heavy border.
 */
import { cva, type VariantProps } from 'class-variance-authority';
import { forwardRef, type HTMLAttributes } from 'react';

import { cn } from './cn';

const alertVariants = cva(
  [
    'relative w-full rounded-lg px-4 py-3 text-sm',
    '[&>svg]:absolute [&>svg]:top-3.5 [&>svg]:left-4 [&>svg~*]:pl-7',
  ],
  {
    variants: {
      variant: {
        default: 'border border-border bg-surface text-fg [&>svg]:text-fg-muted',
        info: 'bg-bg-muted text-fg [&>svg]:text-fg-muted',
        success: 'bg-accent-soft text-fg [&>svg]:text-success',
        warning: 'bg-warning-soft text-warning-text [&>svg]:text-warning',
        destructive: 'bg-danger-soft text-danger [&>svg]:text-danger',
      },
    },
    defaultVariants: { variant: 'default' },
  },
);

/** Props of {@link Alert}. */
export type AlertProps = HTMLAttributes<HTMLDivElement> & VariantProps<typeof alertVariants>;

/** The alert box; put an icon first, then a title and description. */
export const Alert = forwardRef<HTMLDivElement, AlertProps>(
  ({ className, variant, ...props }, ref) => (
    <div ref={ref} role="alert" className={cn(alertVariants({ variant }), className)} {...props} />
  ),
);
Alert.displayName = 'Alert';

/** The alert's one-line title, 14/600. */
export const AlertTitle = forwardRef<HTMLHeadingElement, HTMLAttributes<HTMLHeadingElement>>(
  ({ className, ...props }, ref) => (
    <h5 ref={ref} className={cn('mb-0.5 font-semibold', className)} {...props} />
  ),
);
AlertTitle.displayName = 'AlertTitle';

/** The alert's body text. */
export const AlertDescription = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn('text-sm opacity-90', className)} {...props} />
  ),
);
AlertDescription.displayName = 'AlertDescription';
