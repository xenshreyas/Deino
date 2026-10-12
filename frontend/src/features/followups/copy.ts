/** Every visible string of suggested follow-up emails (NOT-07). */
export const followUpsCopy = {
  banner: {
    title: 'Follow-up email ready',
    body: 'A draft to the people you met, with gaps marked for you to fill.',
    view: 'View draft',
    dismiss: 'Dismiss',
    confirm: 'Dismiss for good? It won’t come back.',
    keep: 'Keep',
    confirmDismiss: 'Dismiss',
  },
  editor: {
    title: 'Follow-up email',
    description: 'Fill the highlighted gaps, then open it in Mail or copy it.',
    to: 'To',
    cc: 'Cc',
    bcc: 'Bcc',
    addressHint: 'Separate addresses with commas',
    subject: 'Subject',
    body: 'Message',
    /** How many `[gaps]` are left to fill. */
    gaps: (count: number) =>
      count === 0
        ? 'No gaps left'
        : count === 1
          ? '1 gap to fill'
          : `${count.toString()} gaps to fill`,
    openInMail: 'Open in Mail',
    copy: 'Copy message',
    copied: 'Message copied',
    sendVia: 'Send via Gmail or Outlook',
    sendViaHint: 'Sending from Deino comes with the Gmail and Outlook integrations.',
    wip: 'WIP',
    close: 'Close',
  },
  dismissed: 'Follow-up dismissed',
  failed: {
    save: 'Could not save the draft',
    mail: 'Could not open Mail',
    copy: 'Could not copy the message',
    dismiss: 'Could not dismiss the follow-up',
  },
} as const;
