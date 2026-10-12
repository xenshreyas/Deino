/**
 * Settings → Privacy & security → App lock (PRV-02): lock on or off, the
 * idle time, locking on wake, and "Lock now". Rust follows each change at
 * once (the settings event updates the lock service).
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { ICON_PROPS, Lock } from '@/ui/icons';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { privacyCopy } from '../copy';
import { useLockActions } from '../queries';
import type { AppLockSettings } from '../types';

const copy = privacyCopy.settings;

/** The idle times offered, in minutes (Rust accepts 1–480). */
export const IDLE_OPTIONS = [1, 5, 15, 30, 60] as const;

/** Saves a change to `privacy.appLock`; failures become a toast. */
function useSaveLock() {
  const patch = usePatchSettings();
  const save = (appLock: Partial<AppLockSettings>) => {
    patch.mutate(
      { privacy: { appLock } },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  return { save, pending: patch.isPending };
}

/** "Lock after": the idle time. */
function IdleSelect({ minutes, onChange }: { minutes: number; onChange: (next: number) => void }) {
  return (
    <Select
      value={String(minutes)}
      onValueChange={(value) => {
        onChange(Number(value));
      }}
    >
      <SelectTrigger id="privacy-lock-idle" aria-label={copy.idleTitle} className="w-32">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {IDLE_OPTIONS.map((option) => (
          <SelectItem key={option} value={String(option)}>
            {copy.idleOption(option)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** "Lock now". */
function LockNowRow() {
  const actions = useLockActions();
  return (
    <div className="flex justify-end px-4 py-3">
      <Button
        variant="secondary"
        size="sm"
        onClick={() => {
          actions.lock.mutate();
        }}
      >
        <Lock {...ICON_PROPS} />
        {copy.lockNow}
      </Button>
    </div>
  );
}

/** The rows that only matter while the lock is on. */
function LockDetails({ lock }: { lock: AppLockSettings }) {
  const { save } = useSaveLock();
  return (
    <>
      <SettingsRow
        label={copy.idleTitle}
        description={copy.idleDescription}
        htmlFor="privacy-lock-idle"
      >
        <IdleSelect
          minutes={lock.idleMinutes}
          onChange={(idleMinutes) => {
            save({ idleMinutes });
          }}
        />
      </SettingsRow>
      <SettingsRow
        label={copy.wakeTitle}
        description={copy.wakeDescription}
        htmlFor="privacy-lock-wake"
      >
        <Switch
          id="privacy-lock-wake"
          checked={lock.lockOnWake}
          onCheckedChange={(lockOnWake) => {
            save({ lockOnWake });
          }}
        />
      </SettingsRow>
      <LockNowRow />
    </>
  );
}

/** The lock switch and, when on, its details. */
function LockCard({ lock }: { lock: AppLockSettings }) {
  const { save, pending } = useSaveLock();
  return (
    <SettingsCard label={copy.lockTitle}>
      <SettingsRow
        label={copy.lockSwitch}
        description={lock.enabled ? copy.lockOn : copy.lockOff}
        htmlFor="privacy-lock"
      >
        <Switch
          id="privacy-lock"
          checked={lock.enabled}
          disabled={pending}
          onCheckedChange={(enabled) => {
            save({ enabled });
          }}
        />
      </SettingsRow>
      {lock.enabled ? <LockDetails lock={lock} /> : null}
    </SettingsCard>
  );
}

/** "App lock", once the settings are loaded. */
export function LockSettings() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.lockTitle} description={copy.lockDescription}>
      {settings.data === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <LockCard lock={settings.data.privacy.appLock} />
      )}
    </SettingsSection>
  );
}
