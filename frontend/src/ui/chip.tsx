/**
 * Meta chip (ui.md §3 Meeting header): 24px pill with a hairline border,
 * 12px muted text and an optional 14px icon. Tags use a colour dot instead.
 */
import { type ButtonHTMLAttributes, forwardRef, type ReactNode } from 'react';

import { cn } from './cn';
import { type Icon, ICON_PROPS } from './icons';

/** Props of {@link Chip}. */
export interface ChipProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: Icon | undefined;
  /** `ghost` chips have no border until hovered ("+ Add tag"). */
  variant?: 'outline' | 'ghost';
  children: ReactNode;
}

const CHIP_BASE =
  'inline-flex h-6 items-center gap-1.5 rounded-full px-2 text-xs whitespace-nowrap text-fg-muted transition-colors duration-(--dur-fast) ease-out';

/** A chip; it is a button so it can open a picker. */
export const Chip = forwardRef<HTMLButtonElement, ChipProps>(
  (
    { icon: ChipIcon, variant = 'outline', className, children, type = 'button', ...props },
    ref,
  ) => (
    <button
      ref={ref}
      type={type}
      className={cn(
        CHIP_BASE,
        variant === 'outline' ? 'border border-border' : 'border border-transparent',
        'enabled:hover:bg-bg-muted enabled:hover:text-fg disabled:cursor-default',
        className,
      )}
      {...props}
    >
      {ChipIcon === undefined ? null : <ChipIcon {...ICON_PROPS} size={14} />}
      {children}
    </button>
  ),
);
Chip.displayName = 'Chip';

/** Props of {@link MetaChip}. */
export interface MetaChipProps {
  icon?: Icon;
  children: ReactNode;
  className?: string;
}

/** A static chip for information that is not interactive (a date, a duration). */
export function MetaChip({ icon: ChipIcon, children, className }: MetaChipProps) {
  return (
    <span className={cn(CHIP_BASE, 'border border-border tabular-nums', className)}>
      {ChipIcon === undefined ? null : <ChipIcon {...ICON_PROPS} size={14} />}
      {children}
    </span>
  );
}
