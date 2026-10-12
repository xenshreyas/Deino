/**
 * Settings → Privacy & security → Encryption (PRV-02): one switch over
 * `privacy.encryptDatabase`, behind a confirmation that says what happens.
 * The file is converted at the next start, so the row compares the setting
 * with the library as it opened and asks for a restart when they differ.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { privacyCopy } from '../copy';
import { useLibraryStatus } from '../queries';
import type { LibraryStatus } from '../types';

const copy = privacyCopy.settings;

/** The row's explanation: what is on disk, and whether a restart is due. */
export function encryptionNote(wanted: boolean, status: LibraryStatus | undefined): string {
  if (status?.kind !== 'ready') return wanted ? copy.encryptedNow : copy.plaintextNow;
  if (status.outcome.kind === 'failed' && status.encrypted !== wanted) {
    return copy.failed(status.outcome.direction);
  }
  if (status.encrypted === wanted) return wanted ? copy.encryptedNow : copy.plaintextNow;
  return wanted ? copy.restartToEncrypt : copy.restartToDecrypt;
}

/** Props of {@link EncryptionConfirm}. */
export interface EncryptionConfirmProps {
  /** The value being confirmed; `null` keeps the dialog closed. */
  next: boolean | null;
  onCancel: () => void;
  onConfirm: (encrypt: boolean) => void;
}

/** "Encrypt the library?" / "Decrypt the library?". */
export function EncryptionConfirm({ next, onCancel, onConfirm }: EncryptionConfirmProps) {
  const encrypt = next === true;
  const lines = encrypt ? copy.confirmEncrypt : copy.confirmDecrypt;
  return (
    <Dialog
      open={next !== null}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
    >
      <DialogContent className="max-w-[440px]">
        <DialogHeader>
          <DialogTitle>{encrypt ? copy.confirmEncryptTitle : copy.confirmDecryptTitle}</DialogTitle>
          <DialogDescription>{lines[0]}</DialogDescription>
        </DialogHeader>
        {lines.slice(1).map((line) => (
          <p key={line} className="text-sm text-fg-muted">
            {line}
          </p>
        ))}
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel}>
            {copy.confirmCancel}
          </Button>
          <Button
            onClick={() => {
              onConfirm(encrypt);
            }}
          >
            {encrypt ? copy.confirmEncryptAction : copy.confirmDecryptAction}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** The switch over the saved setting, with its confirmation. */
function EncryptionCard({ wanted }: { wanted: boolean }) {
  const patch = usePatchSettings();
  const status = useLibraryStatus();
  const [next, setNext] = useState<boolean | null>(null);
  const save = (encryptDatabase: boolean) => {
    setNext(null);
    patch.mutate(
      { privacy: { encryptDatabase } },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  return (
    <SettingsCard label={copy.encryptionTitle}>
      <SettingsRow
        label={copy.encryptTitle}
        description={encryptionNote(wanted, status)}
        htmlFor="privacy-encrypt"
      >
        <Switch
          id="privacy-encrypt"
          checked={wanted}
          disabled={patch.isPending}
          onCheckedChange={setNext}
        />
      </SettingsRow>
      <EncryptionConfirm
        next={next}
        onCancel={() => {
          setNext(null);
        }}
        onConfirm={save}
      />
    </SettingsCard>
  );
}

/** "Encryption", once the settings are loaded. */
export function EncryptionSettings() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.encryptionTitle} description={copy.encryptionDescription}>
      {settings.data === undefined ? (
        <SkeletonText lines={1} />
      ) : (
        <EncryptionCard wanted={settings.data.privacy.encryptDatabase} />
      )}
    </SettingsSection>
  );
}
