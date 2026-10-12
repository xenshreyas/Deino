/**
 * Settings → Consent (CAP-13): the reminder card when recording starts, and
 * the automatic chat message. The chat message needs Accessibility, read
 * from speaker tags' status (the same permission); Deino never asks macOS
 * for it on its own.
 */
import { toast } from 'sonner';

import { type Settings, usePatchSettings, useSettings } from '@/features/settings';
import { useSpeakerTagStatus } from '@/features/speaker-tags';
import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { consentCopy } from '../copy';
import { useOpenAccessibilitySettings } from '../queries';
import { type ChatMessage, ChatMessageCard } from './chat-message-settings';

const copy = consentCopy.settings;

type ConsentSettings = Settings['consent'];

/** Saves a change to the consent section, with a toast on failure. */
function useSaveConsent() {
  const patch = usePatchSettings();
  const save = (consent: { reminder?: boolean; chatMessage?: Partial<ChatMessage> }) => {
    patch.mutate(
      { consent },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  return { save, busy: patch.isPending };
}

/** The reminder's switch. */
function ReminderCard({ consent }: { consent: ConsentSettings }) {
  const { save, busy } = useSaveConsent();
  return (
    <SettingsCard label={copy.reminderTitle}>
      <SettingsRow
        label={copy.reminderTitle}
        description={consent.reminder ? copy.reminderOn : copy.reminderOff}
        htmlFor="consent-reminder"
      >
        <Switch
          id="consent-reminder"
          checked={consent.reminder}
          disabled={busy}
          onCheckedChange={(reminder) => {
            save({ reminder });
          }}
        />
      </SettingsRow>
    </SettingsCard>
  );
}

/** The chat message card over the saved settings and the permission. */
function ChatCard({ consent }: { consent: ConsentSettings }) {
  const { save, busy } = useSaveConsent();
  const status = useSpeakerTagStatus();
  const open = useOpenAccessibilitySettings();
  const needsPermission = (status.data ?? []).some((row) => row.state === 'needsPermission');
  return (
    <ChatMessageCard
      chat={consent.chatMessage}
      busy={busy}
      needsPermission={needsPermission}
      onSave={(chatMessage) => {
        save({ chatMessage });
      }}
      onOpenSettings={() => {
        open.mutate();
      }}
    />
  );
}

/** The page: the reminder, then the chat message. */
export function ConsentSettings() {
  const settings = useSettings();
  if (settings.data === undefined) {
    return (
      <SettingsSection title={copy.reminderSection} description={copy.description}>
        <SkeletonText lines={4} />
      </SettingsSection>
    );
  }
  const { consent } = settings.data;
  return (
    <div className="flex flex-col gap-8">
      <SettingsSection title={copy.reminderSection} description={copy.description}>
        <ReminderCard consent={consent} />
      </SettingsSection>
      <SettingsSection title={copy.chatTitle} description={copy.chatDescription}>
        <ChatCard consent={consent} />
      </SettingsSection>
    </div>
  );
}
