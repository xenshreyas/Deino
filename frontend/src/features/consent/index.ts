/**
 * Consent tools (CAP-13): the reminder card under the recording pill when a
 * recording starts, and Settings → Consent (the reminder and the automatic
 * chat message, which Rust posts in the meeting app's chat).
 */
export { consentApi } from './api';
export { ChatMessageCard, messageError } from './components/chat-message-settings';
export { ConsentCard, type ReminderCopy } from './components/consent-card';
export {
  ConsentReminder,
  type ReminderState,
  shouldRemind,
  useConsentReminder,
} from './components/consent-reminder';
export { ConsentSettings } from './components/consent-settings';
export { consentCopy } from './copy';
export { consentKeys, reminderQuery } from './queries';
