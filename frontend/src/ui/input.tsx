/**
 * Text input and textarea (ui.md §3 Inputs): surface fill, hairline border
 * that strengthens on hover, accent border and soft ring on focus.
 */
import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from 'react';

import { cn } from './cn';

/** Shared field look; also used by selects and input groups. */
export const fieldClassName = cn(
  'w-full rounded-md border border-border bg-surface text-sm text-fg',
  'transition-[border-color,box-shadow] duration-(--dur-fast) ease-out',
  'placeholder:text-fg-subtle hover:border-border-strong',
  'focus-visible:border-accent focus-visible:shadow-[0_0_0_3px_color-mix(in_srgb,var(--accent)_20%,transparent)] focus-visible:outline-none',
  'aria-invalid:border-danger',
  'disabled:cursor-not-allowed disabled:bg-bg-muted disabled:text-fg-subtle',
);

/** A 32px single-line text field. */
export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, type = 'text', ...props }, ref) => (
    <input
      ref={ref}
      type={type}
      className={cn(fieldClassName, 'h-8 px-2.5', className)}
      {...props}
    />
  ),
);
Input.displayName = 'Input';

/** A multi-line text field that keeps the input look. */
export const Textarea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement>
>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={cn(fieldClassName, 'min-h-20 px-2.5 py-2', className)}
    {...props}
  />
));
Textarea.displayName = 'Textarea';
