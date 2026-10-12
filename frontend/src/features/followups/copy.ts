/** Every visible string of suggested follow-up emails (NOT-07). */
export const followUpsCopy = {
  chip: {
    title: 'Follow-up email ready',
    hint: 'A draft to the people you met, with gaps marked for you to fill.',
    view: 'View draft',
    dismiss: 'Dismiss for good',
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
    close: 'Close',
  },
  failed: {
    save: 'Couldn’t save the draft',
    mail: 'Couldn’t open Mail',
    copy: 'Couldn’t copy the message',
    dismiss: 'Couldn’t dismiss the follow-up',
  },
} as const;
