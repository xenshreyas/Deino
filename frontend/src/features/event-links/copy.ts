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
  linkedTo: (title: string) => `Linked to ${title}`,
  unlinked: 'Unlinked from the calendar event',
  untitled: 'Untitled event',
  people: (count: number) => (count === 1 ? '1 person' : `${count} people`),
  attendees: (added: number, removed: number) => attendeeSummary(added, removed),

  eventSection: 'Calendar event',
  guests: 'Guests',
  moreGuests: (count: number) => `and ${count} more`,
  previousChip: 'Previous',
  previousInSeries: 'Previous in series',
  relatedSection: 'Related meetings',
  openPrevious: (title: string) => `Open the previous meeting, ${title}`,
  by: {
    series: 'Same series',
    titleAndPeople: 'Same title and people',
  },
} as const;

/** "2 attendees added, 1 removed", or `undefined` when nobody changed. */
function attendeeSummary(added: number, removed: number): string | undefined {
  const parts = [
    added > 0 ? `${added} ${added === 1 ? 'attendee' : 'attendees'} added` : null,
    removed > 0 ? `${removed} removed` : null,
  ].filter((part) => part !== null);
  return parts.length === 0 ? undefined : parts.join(', ');
}
