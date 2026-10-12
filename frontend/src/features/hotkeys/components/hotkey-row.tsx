/**
 * One hotkey in Settings → Shortcuts: what it does, its combination, and
 * Change / Turn off / Reset. While listening, the capture button takes the
 * next combination (Esc cancels). Presentational.
 */
import type { KeyboardEvent } from 'react';

import { Button } from '@/ui/button';
import { Kbd } from '@/ui/kbd';

import { acceleratorFromPress, formatAccelerator } from '../accelerator';
import { hotkeysCopy as copy } from '../copy';
import type { Hotkey } from '../types';

/** Props of {@link HotkeyRow}. */
export interface HotkeyRowProps {
  hotkey: Hotkey;
  /** Waiting for the new combination. */
  listening: boolean;
  /** A change is being saved. */
  saving: boolean;
  /** Why the last change was refused, if it was. */
  error: string | null;
  onListen: () => void;
  onCancel: () => void;
  /** Saves `accelerator`, or turns the hotkey off with `null`. */
  onChange: (accelerator: string | null) => void;
}

/** Why the hotkey does not work, or the refused change's reason. */
function Problem({ hotkey, error }: { hotkey: Hotkey; error: string | null }) {
  const reason = error ?? (hotkey.state.kind === 'failed' ? hotkey.state.reason : null);
  if (reason === null) return null;
  return (
    <p role="alert" className="mt-1 text-xs text-danger">
      {reason}
    </p>
  );
}

/** The button that listens for the new combination. */
function Capture({ onCancel, onChange }: Pick<HotkeyRowProps, 'onCancel' | 'onChange'>) {
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    event.preventDefault();
    const bare = !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey;
    if (event.code === 'Escape' && bare) {
      onCancel();
      return;
    }
    const accelerator = acceleratorFromPress(event);
    if (accelerator !== null) onChange(accelerator);
  };
  return (
    <Button
      ref={(button) => button?.focus()}
      variant="secondary"
      size="sm"
      aria-label={copy.listeningHint}
      onKeyDown={onKeyDown}
      onBlur={onCancel}
    >
      {copy.listening}
    </Button>
  );
}

/** The combination as key caps, or "Off". */
function Combination({ accelerator }: { accelerator: string | null }) {
  if (accelerator === null) return <span className="text-xs text-fg-subtle">{copy.off}</span>;
  return <Kbd className="px-1.5">{formatAccelerator(accelerator)}</Kbd>;
}

/** A quiet text button that sets the hotkey to `value` (`null`: off). */
function SetButton({ label, value, saving, onChange }: SetButtonProps) {
  return (
    <Button
      variant="ghost"
      size="sm"
      disabled={saving}
      onClick={() => {
        onChange(value);
      }}
    >
      {label}
    </Button>
  );
}

/** Props of {@link SetButton}. */
interface SetButtonProps extends Pick<HotkeyRowProps, 'saving' | 'onChange'> {
  label: string;
  value: string | null;
}

/** One hotkey. */
export function HotkeyRow(props: HotkeyRowProps) {
  const { hotkey, listening, saving, error, onListen } = props;
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-sm text-fg">{hotkey.label}</p>
        <p className="mt-0.5 text-xs text-fg-muted">{copy.descriptions[hotkey.action]}</p>
        <Problem hotkey={hotkey} error={error} />
      </div>
      <div className="flex items-center gap-2">
        {listening ? <Capture {...props} /> : <Combination accelerator={hotkey.accelerator} />}
        <Button variant="ghost" size="sm" disabled={saving || listening} onClick={onListen}>
          {copy.change}
        </Button>
        {hotkey.accelerator === null ? null : (
          <SetButton {...props} label={copy.turnOff} value={null} />
        )}
        {hotkey.accelerator === hotkey.defaultAccelerator ? null : (
          <SetButton {...props} label={copy.reset} value={hotkey.defaultAccelerator} />
        )}
      </div>
    </div>
  );
}
