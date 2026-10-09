/**
 * The tag colour picker: six swatches from the speaker palette as a radio
 * group (arrow keys move the choice and the focus, as in a native group).
 */
import type { KeyboardEvent } from 'react';

import { cn } from '@/ui/cn';
import { Label } from '@/ui/controls';

import { tagsCopy } from '../copy';
import { PALETTE, type PaletteSlot } from '../palette';
import type { PaletteColor } from '../types';

/** Props of {@link ColorField}. */
export interface ColorFieldProps {
  value: PaletteColor;
  onChange: (color: PaletteColor) => void;
}

/** Arrow keys and the step they move the choice by. */
const ARROW_STEPS: Partial<Record<string, number>> = {
  ArrowRight: 1,
  ArrowDown: 1,
  ArrowLeft: -1,
  ArrowUp: -1,
};

/** The palette colour `step` places from `color`, wrapping around. */
export function nextColor(color: PaletteColor, step: number): PaletteColor {
  const index = PALETTE.findIndex((slot) => slot.color === color);
  const next = PALETTE.at((index + step) % PALETTE.length);
  return next?.color ?? color;
}

/** Moves the choice with the arrow keys and focuses the new swatch. */
function arrowKeys(value: PaletteColor, onChange: (color: PaletteColor) => void) {
  return (event: KeyboardEvent<HTMLButtonElement>) => {
    const step = ARROW_STEPS[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const next = nextColor(value, step);
    onChange(next);
    const group = event.currentTarget.parentElement;
    group?.querySelector<HTMLElement>(`[data-color="${next}"]`)?.focus();
  };
}

interface SwatchProps {
  slot: PaletteSlot;
  checked: boolean;
  onSelect: () => void;
  onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

/** One round swatch; the chosen one sits in a hairline ring. */
function Swatch({ slot, checked, onSelect, onKeyDown }: SwatchProps) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      aria-label={slot.name}
      data-color={slot.color}
      tabIndex={checked ? 0 : -1}
      onClick={onSelect}
      onKeyDown={onKeyDown}
      className={cn(
        'flex size-7 items-center justify-center rounded-full transition-shadow duration-(--dur-fast)',
        checked
          ? 'ring-2 ring-border-strong ring-offset-2 ring-offset-surface-raised'
          : 'hover:ring-1 hover:ring-border',
      )}
    >
      <span className={cn('size-4 rounded-full', slot.dot)} />
    </button>
  );
}

/** The six swatches. */
export function ColorField({ value, onChange }: ColorFieldProps) {
  const onKeyDown = arrowKeys(value, onChange);
  return (
    <div className="grid gap-1.5">
      <Label id="tag-colour-label">{tagsCopy.colorLabel}</Label>
      <div role="radiogroup" aria-labelledby="tag-colour-label" className="flex gap-2">
        {PALETTE.map((slot) => (
          <Swatch
            key={slot.color}
            slot={slot}
            checked={slot.color === value}
            onSelect={() => {
              onChange(slot.color);
            }}
            onKeyDown={onKeyDown}
          />
        ))}
      </div>
    </div>
  );
}
