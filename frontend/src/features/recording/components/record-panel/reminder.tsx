/**
 * At most one reminder under the pill when a recording starts (UX-28):
 * Consent's "Let people know" card (CAP-13), which also mentions the
 * shared screen while it is captured; or, with that reminder off, screen
 * capture's own card (CAP-09). "Got it" and "Don't show again" answer for
 * both when they share a card.
 */
import {
  ConsentCard,
  consentCopy,
  type ReminderState,
  useConsentReminder,
} from '@/features/consent';
import { type ScreenReminderState, screenCopy, useScreenReminder } from '@/features/screen';
import type { MeetingId } from '@/lib/ipc/generated/meetings';

/**
 * The consent card's answers; while the screen is captured the card speaks
 * for the screen reminder too, so they answer it as well (it would
 * otherwise pop up on its own once the consent card is gone).
 */
function both(
  consent: ReminderState,
  screen: ScreenReminderState,
): Pick<ReminderState, 'dismiss' | 'never'> {
  return {
    dismiss: () => {
      consent.dismiss();
      if (screen.captured) screen.dismiss();
    },
    never: () => {
      consent.never();
      if (screen.captured) screen.never();
    },
  };
}

/** The reminder for the recording of `meetingId` (`null`: nothing records). */
export function RecordingReminder({ meetingId }: { meetingId: MeetingId | null }) {
  const consent = useConsentReminder(meetingId);
  const screen = useScreenReminder(meetingId);
  if (consent.shown) {
    const answers = both(consent, screen);
    const copy = screen.captured
      ? { ...consentCopy.reminder, body: consentCopy.reminder.bodyWithScreen }
      : consentCopy.reminder;
    return (
      <ConsentCard
        copy={copy}
        busy={consent.busy || screen.busy}
        onDismiss={answers.dismiss}
        onNever={answers.never}
      />
    );
  }
  if (!screen.shown) return null;
  return (
    <ConsentCard
      copy={screenCopy.reminder}
      busy={screen.busy}
      onDismiss={screen.dismiss}
      onNever={screen.never}
    />
  );
}
