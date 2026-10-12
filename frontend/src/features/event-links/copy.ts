/** Every visible string of the event-links feature. */
export const eventLinksCopy = {
  linkEvent: 'Link event',
  changeEvent: 'Change calendar event',
  pickerLabel: 'Calendar event',
  searchPlaceholder: 'Search events',
  linkedGroup: 'Linked',
  suggestedGroup: 'Around this meeting',
  unlink: 'Unlink event',
  noSuggestions: 'No other events in the day before this meeting.',
  noMatch: 'No matching events',
  loadFailed: 'Could not load your calendar.',
  linkFailed: 'Could not link the event.',
  unlinkFailed: 'Could not unlink the event.',
  untitled: 'Untitled event',
  people: (count: number) => (count === 1 ? '1 person' : `${count} people`),

  eventSection: 'Calendar event',
  guests: 'Guests',
  moreGuests: (count: number) => `and ${count} more`,
  previousInSeries: 'Previous in series',
  relatedSection: 'Related meetings',
  by: {
    series: 'Same series',
    titleAndPeople: 'Same title and people',
    people: 'Same people',
  },
} as const;
