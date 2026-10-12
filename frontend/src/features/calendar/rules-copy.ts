/** ORG-02's strings in the calendar feature: the reminder window and its settings, and auto-record. */
export const rulesCopy = {
  reminder: {
    label: 'Meeting starting soon',
    startsIn: (minutes: number) =>
      minutes <= 1 ? 'Starts in a minute' : `Starts in ${minutes} minutes`,
    started: 'Starting now',
    others: (count: number) => `+${count}`,
    joinAndRecord: 'Join & record',
    recordOnly: 'Record only',
    dismiss: 'Dismiss',
    failed: 'Could not start. Try again from Home.',
  },
  reminders: {
    title: 'Meeting reminders',
    description:
      'A small window a minute before each meeting with two or more people, with Join & record.',
    enabled: 'Remind me a minute before meetings',
    enabledOn: 'Deino’s Do Not Disturb keeps it quiet.',
    enabledOff: 'No reminder windows.',
    calendars: 'Remind for',
    remindFor: (title: string) => `Remind for ${title}`,
    saveFailed: 'Could not save the reminder settings.',
  },
  autoRecord: {
    title: 'Auto-record',
    description:
      'Which meetings record on their own when the call starts. A choice on an event or series in Coming up wins.',
    internal: 'Internal meetings',
    internalHint: 'Only people from your own domains.',
    external: 'External meetings',
    externalHint: 'Someone from outside your domains is invited.',
    organizedByOthers: 'Meetings others organise',
    organizedByOthersHint: 'Off: only meetings you organised.',
    notAccepted: 'Meetings I have not accepted',
    notAcceptedHint: 'Tentative or not answered yet.',
    saveFailed: 'Could not save auto-record.',
  },
} as const;
