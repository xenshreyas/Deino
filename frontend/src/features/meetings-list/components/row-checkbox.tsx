/**
 * A row's selection checkbox (ui.md §3 Inputs, "Checkbox: 16px r4, on
 * --accent"). It shows on hover, on focus and while anything is selected;
 * Shift-click selects the range from the last click.
 */
import { cn } from '@/ui/cn';

/** Props of {@link RowCheckbox}. */
export interface RowCheckboxProps {
  checked: boolean;
  /** The accessible name: what ticking it does. */
  label: string;
  /** Shown even without hover (something is selected). */
  visible: boolean;
  /** A click; `range` when Shift was held. */
  onToggle: (range: boolean) => void;
}

/** The checkbox in a 32px hit area at the row's start. */
export function RowCheckbox({ checked, label, visible, onToggle }: RowCheckboxProps) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={label}
      onClick={(event) => {
        event.stopPropagation();
        onToggle(event.shiftKey);
      }}
      className={cn(
        'group/check flex h-full w-8 shrink-0 items-center justify-center rounded-md',
        'focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-focus-ring',
        visible ? 'opacity-100' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100',
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          'flex size-4 items-center justify-center rounded-xs border transition-colors duration-(--dur-fast) ease-out',
          checked
            ? 'border-accent bg-accent text-accent-fg'
            : 'border-border-strong bg-control group-hover/check:border-fg-subtle',
        )}
      >
        <svg viewBox="0 0 16 16" className={cn('size-3', !checked && 'invisible')}>
          <path
            d="M3.5 8.5 6.5 11.5 12.5 4.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    </button>
  );
}
