/**
 * Settings → Privacy & security (PRV-02): encryption at rest, the app lock,
 * retention (PRV-01), and how to protect recordings, which Deino does not
 * encrypt (`docs/setup/encrypted-recordings.md`, summarised inline).
 */
import { RetentionSettings } from '@/features/retention';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';

import { privacyCopy } from '../copy';
import { EncryptionSettings } from './encryption-settings';
import { LockSettings } from './lock-settings';

const copy = privacyCopy.settings;

/** "Recordings": the encrypted-volume advice. */
function RecordingsNote() {
  return (
    <SettingsSection title={copy.recordingsTitle} description={copy.recordingsDescription}>
      <SettingsCard label={copy.recordingsTitle}>
        <div className="flex flex-col gap-2 px-4 py-3 text-sm text-fg-muted">
          {copy.recordingsBody.map((line) => (
            <p key={line}>{line}</p>
          ))}
        </div>
      </SettingsCard>
    </SettingsSection>
  );
}

/** The whole page. */
export function PrivacySettings() {
  return (
    <div className="flex flex-col gap-8">
      <EncryptionSettings />
      <LockSettings />
      <RetentionSettings />
      <RecordingsNote />
    </div>
  );
}
