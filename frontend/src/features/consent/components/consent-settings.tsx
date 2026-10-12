/**
 * Settings → Recording → Tell people (CAP-13): the reminder when recording
 * starts, then under "More options" the automatic chat message. The chat
 * message needs Accessibility, read from speaker tags' status (the same
 * permission); while it is missing the section links to the Permissions
 * card (UX-10). Deino never asks macOS for it on its own.
 */
import { toast } from 'sonner';

import { PermissionLink, type Settings, usePatchSettings, useSettings } from '@/features/settings';
import { useSpeakerTagStatus } from '@/features/speaker-tags';
import { Switch } from '@/ui/controls';
import { MoreOptions } from '@/ui/more-options';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { consentCopy } from '../copy';
import { anyAppOn, type ChatMessage, ChatMessageCard } from './chat-message-settings';

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

/** The chat message card over the saved settings, and the permission link. */
function ChatCard({ consent }: { consent: ConsentSettings }) {
  const { save, busy } = useSaveConsent();
  const status = useSpeakerTagStatus();
  const needsPermission = (status.data ?? []).some((row) => row.state === 'needsPermission');
  return (
    <div className="flex flex-col gap-2">
      <h4 className="text-sm font-semibold text-fg">{copy.chatTitle}</h4>
      <p className="text-xs text-fg-muted">{copy.chatDescription}</p>
      <ChatMessageCard
        chat={consent.chatMessage}
        busy={busy}
        onSave={(chatMessage) => {
          save({ chatMessage });
        }}
      />
      {needsPermission && anyAppOn(consent.chatMessage) ? (
        <PermissionLink permission={copy.permission} />
      ) : null}
    </div>
  );
}

/** The section: the reminder, then the chat message under "More options". */
export function ConsentSettings() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.reminderSection} description={copy.description}>
      {settings.data === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <>
          <ReminderCard consent={settings.data.consent} />
          <MoreOptions>
            <ChatCard consent={settings.data.consent} />
          </MoreOptions>
        </>
      )}
    </SettingsSection>
  );
}
