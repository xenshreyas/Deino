/**
 * Settings → Appearance: three cards (System / Light / Dark), each with a
 * miniature window drawn in that theme. The System card is split diagonally
 * (the anarlog appearance picker's idea, MIT).
 */
import { cn } from '@/ui/cn';

import { appearanceCopy } from '../copy';
import type { ThemePreference } from '../theme';

const CHOICES: readonly { value: ThemePreference; label: string }[] = [
  { value: 'system', label: appearanceCopy.system },
  { value: 'light', label: appearanceCopy.light },
  { value: 'dark', label: appearanceCopy.dark },
];

/** Props of {@link ThemePicker}. */
export interface ThemePickerProps {
  value: ThemePreference;
  onChange: (preference: ThemePreference) => void;
}

/** A radio group of theme cards. */
export function ThemePicker({ value, onChange }: ThemePickerProps) {
  return (
    <div
      role="radiogroup"
      aria-label={appearanceCopy.themeLabel}
      className="grid grid-cols-3 gap-3"
    >
      {CHOICES.map((choice) => (
        <button
          key={choice.value}
          type="button"
          role="radio"
          aria-checked={value === choice.value}
          onClick={() => {
            onChange(choice.value);
          }}
          className="group flex flex-col gap-2 text-left"
        >
          <ThemePreview theme={choice.value} selected={value === choice.value} />
          <span className="text-sm text-fg-muted group-aria-checked:font-semibold group-aria-checked:text-fg">
            {choice.label}
          </span>
        </button>
      ))}
    </div>
  );
}

/** The miniature window for one choice. */
function ThemePreview({ theme, selected }: { theme: ThemePreference; selected: boolean }) {
  return (
    <div
      className={cn(
        'relative h-20 overflow-hidden rounded-lg border transition-[border-color,box-shadow] duration-(--dur-fast)',
        selected
          ? 'border-accent shadow-[0_0_0_1px_var(--accent)]'
          : 'border-border group-hover:border-border-strong',
      )}
    >
      <MiniWindow theme={theme === 'dark' ? 'dark' : 'light'} />
      {theme === 'system' ? (
        <div className="absolute inset-0 [clip-path:polygon(100%_0,100%_100%,0_100%)]">
          <MiniWindow theme="dark" />
        </div>
      ) : null}
    </div>
  );
}

/** A sidebar, a title line and two text lines, rendered in one theme. */
function MiniWindow({ theme }: { theme: 'light' | 'dark' }) {
  return (
    <div data-theme={theme} className="flex size-full bg-bg">
      <div className="w-1/4 border-r border-border bg-bg-subtle" />
      <div className="flex flex-1 flex-col gap-1.5 p-2.5">
        <div className="h-1.5 w-1/2 rounded-full bg-fg" />
        <div className="h-1 w-5/6 rounded-full bg-fg-subtle" />
        <div className="h-1 w-2/3 rounded-full bg-fg-subtle" />
        <div className="mt-auto h-1.5 w-1/3 rounded-full bg-accent" />
      </div>
    </div>
  );
}
