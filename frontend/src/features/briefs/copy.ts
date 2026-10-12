/** Every visible string of pre-meeting briefs (NOT-08). */
export const briefsCopy = {
  title: 'Brief',
  /** Collapsed card: how many points it holds. */
  points: (count: number) => (count === 1 ? '1 point' : `${count.toString()} points`),
  expand: 'Show brief',
  collapse: 'Hide brief',
  notUseful: 'Not useful',
  thanks: 'Thanks, noted',
  feedbackFailed: 'Could not save your feedback',
  /** Citation chips. */
  citation: {
    event: 'Invitation',
  },
  settings: {
    title: 'Pre-meeting briefs',
    description:
      'A few cited points before each meeting, from your earlier meetings, open action items and notes about the people. Prepared at 06:00 and an hour before; nothing leaves your Mac except the request to your AI provider.',
    enabledTitle: 'Prepare briefs',
    enabledOn: 'Briefs are prepared for upcoming meetings',
    enabledOff: 'No briefs are prepared',
    audienceTitle: 'Which meetings',
    audience: {
      external: 'With people outside your organisation',
      groups: 'Every meeting with two or more people',
    },
    saveFailed: 'Could not save the setting',
  },
} as const;
