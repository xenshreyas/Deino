/**
 * Icon-only button: 28×28 ghost with an always-present tooltip that names the
 * action and its shortcut (ui.md §3 Buttons).
 */
import { type ButtonHTMLAttributes, forwardRef } from 'react';

import { buttonVariants } from './button';
import { cn } from './cn';
import { type Icon, ICON_PROPS } from './icons';
import { Hint } from './tooltip';

/** Props of {@link IconButton}. */
export interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  /** The icon component from `@/ui/icons`. */
  icon: Icon;
  /** Accessible name, also shown in the tooltip. */
  label: string;
  /** Shortcut shown in the tooltip, in display form (`⌘N`). */
  shortcut?: string | undefined;
  tooltipSide?: 'top' | 'right' | 'bottom' | 'left' | undefined;
}

/** A ghost icon button with its tooltip. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon: IconComponent, label, shortcut, tooltipSide, className, ...props }, ref) => (
    <Hint label={label} shortcut={shortcut} side={tooltipSide}>
      <button
        ref={ref}
        type="button"
        aria-label={label}
        className={cn(buttonVariants({ variant: 'ghost', size: 'icon' }), className)}
        {...props}
      >
        <IconComponent {...ICON_PROPS} />
      </button>
    </Hint>
  ),
);
IconButton.displayName = 'IconButton';
