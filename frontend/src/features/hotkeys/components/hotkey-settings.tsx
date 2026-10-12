/**
 * Settings → Shortcuts (DEV-05): the system-wide hotkeys (start or stop
 * recording, quick note, Ask), each changeable, off-able and resettable.
 * Rust validates a new combination and registers it before saving it, so
 * a refused one shows its reason under the row and the old one keeps
 * working.
 */
import { useState } from 'react';

import { IpcError } from '@/lib/ipc';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { hotkeysCopy as copy } from '../copy';
import { useHotkeys, useSetHotkey } from '../queries';
import type { Hotkey, HotkeyAction } from '../types';
import { HotkeyRow } from './hotkey-row';

/** Props of {@link HotkeysCard}. */
export interface HotkeysCardProps {
  hotkeys: readonly Hotkey[];
  saving: boolean;
  /** The refused change, by action. */
  refused: { action: HotkeyAction; message: string } | null;
  onChange: (action: HotkeyAction, accelerator: string | null) => void;
}

/** The rows; tracks which one is listening. Presentational over its props. */
export function HotkeysCard({ hotkeys, saving, refused, onChange }: HotkeysCardProps) {
  const [listening, setListening] = useState<HotkeyAction | null>(null);
  return (
    <SettingsCard label={copy.section.title}>
      {hotkeys.map((hotkey) => (
        <HotkeyRow
          key={hotkey.action}
          hotkey={hotkey}
          listening={listening === hotkey.action}
          saving={saving}
          error={refused?.action === hotkey.action ? refused.message : null}
          onListen={() => {
            setListening(hotkey.action);
          }}
          onCancel={() => {
            setListening(null);
          }}
          onChange={(accelerator) => {
            setListening(null);
            onChange(hotkey.action, accelerator);
          }}
        />
      ))}
    </SettingsCard>
  );
}

/** The rows once loaded, a skeleton before, an inline error on failure. */
function HotkeysBody() {
  const hotkeys = useHotkeys();
  const change = useSetHotkey();
  if (hotkeys.data !== undefined) {
    const refused = change.isError
      ? { action: change.variables.action, message: IpcError.from(change.error).message }
      : null;
    return (
      <HotkeysCard
        hotkeys={hotkeys.data}
        saving={change.isPending}
        refused={refused}
        onChange={(action, accelerator) => {
          change.mutate({ action, accelerator });
        }}
      />
    );
  }
  if (hotkeys.isError) {
    return (
      <p role="alert" className="text-xs text-danger">
        {copy.loadFailed}
      </p>
    );
  }
  return <SkeletonText lines={3} />;
}

/** The page: the hotkeys, then a note on `deino://` links. */
export function HotkeySettings() {
  return (
    <SettingsSection title={copy.section.title} description={copy.section.description}>
      <HotkeysBody />
      <p className="px-1 text-xs text-fg-muted">{copy.links}</p>
    </SettingsSection>
  );
}
