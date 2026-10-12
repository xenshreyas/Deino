/**
 * Settings → Consent → Automatic chat message (CAP-13): one switch per
 * meeting app (all off by default), the message, and the way to System
 * Settings when Accessibility is missing. Presentational: the page passes
 * the saved settings and how to save them.
 */
import type { Settings } from '@/features/settings';
import type { CallApp } from '@/features/speaker-tags';
import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { Textarea } from '@/ui/input';
import { SettingsCard, SettingsRow } from '@/ui/settings-section';

import { consentCopy } from '../copy';

const copy = consentCopy.settings;

/** The longest message Rust accepts (`MAX_CHAT_MESSAGE_CHARS`). */
export const MAX_MESSAGE_CHARS = 500;

/** The apps, in the order they are listed. */
const APPS: readonly CallApp[] = ['zoom', 'teams', 'meet'];

/** The saved chat-message settings. */
export type ChatMessage = Settings['consent']['chatMessage'];

/** Props of {@link ChatMessageCard}. */
export interface ChatMessageCardProps {
  chat: ChatMessage;
  busy: boolean;
  /** Accessibility is missing, so nothing can be posted yet. */
  needsPermission: boolean;
  onSave: (next: Partial<ChatMessage>) => void;
  onOpenSettings: () => void;
}

/** Whether `message` can be saved while `chat`'s switches are as they are. */
export function messageError(chat: ChatMessage, message: string): string | null {
  const anyOn = APPS.some((app) => chat[app]);
  return anyOn && message.trim() === '' ? copy.messageEmpty : null;
}

/** The message, saved when the field loses focus. */
function MessageField({
  chat,
  busy,
  onSave,
}: Omit<ChatMessageCardProps, 'needsPermission' | 'onOpenSettings'>) {
  return (
    <div className="flex flex-col gap-1.5 px-4 py-3">
      <label htmlFor="consent-message" className="text-sm text-fg">
        {copy.messageTitle}
      </label>
      <Textarea
        id="consent-message"
        key={chat.message}
        defaultValue={chat.message}
        maxLength={MAX_MESSAGE_CHARS}
        rows={3}
        disabled={busy}
        onBlur={(event) => {
          const message = event.currentTarget.value;
          if (message !== chat.message && messageError(chat, message) === null) onSave({ message });
        }}
      />
      <p className="text-xs text-fg-muted">{copy.messageHint(MAX_MESSAGE_CHARS)}</p>
    </div>
  );
}

/** Accessibility is missing: the hint and the way to System Settings. */
function PermissionRow({ onOpenSettings }: { onOpenSettings: () => void }) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <p className="text-xs text-fg-muted">{copy.permissionHint}</p>
      <Button variant="secondary" size="sm" onClick={onOpenSettings}>
        {copy.openSettings}
      </Button>
    </div>
  );
}

/** The switches, the message and, when needed, the permission hint. */
export function ChatMessageCard(props: ChatMessageCardProps) {
  const { chat, busy, needsPermission, onSave, onOpenSettings } = props;
  return (
    <SettingsCard label={copy.chatTitle}>
      {APPS.map((app) => (
        <SettingsRow
          key={app}
          label={consentCopy.apps[app].name}
          description={consentCopy.apps[app].where}
          htmlFor={`consent-${app}`}
        >
          <Switch
            id={`consent-${app}`}
            checked={chat[app]}
            disabled={busy || (!chat[app] && chat.message.trim() === '')}
            onCheckedChange={(on) => {
              onSave({ [app]: on });
            }}
          />
        </SettingsRow>
      ))}
      <MessageField chat={chat} busy={busy} onSave={onSave} />
      {needsPermission && APPS.some((app) => chat[app]) ? (
        <PermissionRow onOpenSettings={onOpenSettings} />
      ) : null}
    </SettingsCard>
  );
}
