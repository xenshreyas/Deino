/**
 * Dialog (Radix), reserved for destructive or irreversible confirmations and
 * focused tasks (ui.md §3 Dialog): r12, raised surface, `--overlay` scrim with
 * no blur, 200ms scale-and-fade.
 */
import * as DialogPrimitive from '@radix-ui/react-dialog';
import {
  type ComponentPropsWithoutRef,
  type ElementRef,
  forwardRef,
  type HTMLAttributes,
} from 'react';

import { cn } from './cn';
import { ICON_PROPS, X } from './icons';

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogPortal = DialogPrimitive.Portal;
export const DialogClose = DialogPrimitive.Close;

/** The scrim behind a dialog. */
export const DialogOverlay = forwardRef<
  ElementRef<typeof DialogPrimitive.Overlay>,
  ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay
    ref={ref}
    className={cn(
      'fixed inset-0 z-(--z-overlay) bg-overlay',
      'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0',
      'duration-(--dur-slow)',
      className,
    )}
    {...props}
  />
));
DialogOverlay.displayName = 'DialogOverlay';

/** Props of {@link DialogContent}. */
export interface DialogContentProps extends ComponentPropsWithoutRef<
  typeof DialogPrimitive.Content
> {
  /** Hide the corner close button (the footer has its own). */
  hideClose?: boolean;
}

/** The dialog panel, 440px by default; pass `max-w-*` for md (560) or lg (720). */
export const DialogContent = forwardRef<
  ElementRef<typeof DialogPrimitive.Content>,
  DialogContentProps
>(({ className, children, hideClose = false, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      className={cn(
        'fixed top-1/2 left-1/2 z-(--z-dialog) grid max-h-[85vh] w-[calc(100%-2rem)] max-w-[440px] -translate-x-1/2 -translate-y-1/2 gap-4 overflow-y-auto',
        'rounded-lg bg-surface-raised p-6 text-fg shadow-dialog',
        'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98]',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-[0.98]',
        'duration-(--dur-slow) ease-out',
        className,
      )}
      {...props}
    >
      {children}
      {hideClose ? null : (
        <DialogPrimitive.Close className="absolute top-4 right-4 flex size-7 items-center justify-center rounded-sm text-fg-muted transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg">
          <X {...ICON_PROPS} />
          <span className="sr-only">Close</span>
        </DialogPrimitive.Close>
      )}
    </DialogPrimitive.Content>
  </DialogPortal>
));
DialogContent.displayName = 'DialogContent';

/** Title and description block. */
export function DialogHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('flex flex-col gap-1 pr-8', className)} {...props} />;
}

/** Right-aligned actions: Cancel (ghost) then the primary action. */
export function DialogFooter({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('flex justify-end gap-2 pt-2', className)} {...props} />;
}

/** The dialog title, 16/600. */
export const DialogTitle = forwardRef<
  ElementRef<typeof DialogPrimitive.Title>,
  ComponentPropsWithoutRef<typeof DialogPrimitive.Title>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Title
    ref={ref}
    className={cn('text-base font-semibold text-fg', className)}
    {...props}
  />
));
DialogTitle.displayName = 'DialogTitle';

/** The description under the title, 14px muted. */
export const DialogDescription = forwardRef<
  ElementRef<typeof DialogPrimitive.Description>,
  ComponentPropsWithoutRef<typeof DialogPrimitive.Description>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Description
    ref={ref}
    className={cn('text-sm text-fg-muted', className)}
    {...props}
  />
));
DialogDescription.displayName = 'DialogDescription';
