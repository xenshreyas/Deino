/**
 * The completion checkbox (ui.md §3 Inputs, "Checkbox: 16px r4, on
 * --accent"). Ticking fills it with the accent and draws the check stroke;
 * without motion (prefers-reduced-motion) the state simply changes.
 */
import { cn } from '@/ui/cn';

/** Props of {@link ItemCheckbox}. */
export interface ItemCheckboxProps {
  checked: boolean;
  /** The accessible name: what ticking it does. */
  label: string;
  onToggle: () => void;
  className?: string;
}

/** A round-cornered box that fills and draws a check when ticked. */
export function ItemCheckbox({ checked, label, onToggle, className }: ItemCheckboxProps) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={label}
      onClick={(event) => {
        event.stopPropagation();
        onToggle();
      }}
      className={cn(
        'group/check relative flex size-4 shrink-0 items-center justify-center rounded-xs border',
        'transition-[background-color,border-color,transform] duration-(--dur-fast) ease-out',
        'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
        'active:scale-90 motion-reduce:active:scale-100',
        checked
          ? 'border-accent bg-accent text-accent-fg'
          : 'border-border-strong bg-control hover:border-fg-subtle',
        className,
      )}
    >
      <svg viewBox="0 0 16 16" className="size-3" aria-hidden="true">
        <path
          d="M3.5 8.5 6.5 11.5 12.5 4.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          // A unit path length lets the dash offset draw the whole stroke.
          pathLength={1}
          strokeDasharray={1}
          strokeDashoffset={checked ? 0 : 1}
          opacity={checked ? 1 : 0}
          className="transition-[stroke-dashoffset] duration-(--dur-slow) ease-out motion-reduce:transition-none"
        />
      </svg>
    </button>
  );
}
