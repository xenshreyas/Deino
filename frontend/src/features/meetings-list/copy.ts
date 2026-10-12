/** Every string the Meetings page and the Trash show. */

const plural = (count: number, one: string, many: string) =>
  `${String(count)} ${count === 1 ? one : many}`;

export const copy = {
  title: 'Meetings',
  newNote: 'New note',
  loadError: 'Your meetings could not be loaded.',
  retry: 'Try again',
  loadMore: 'Loading more…',
  saved: 'Saved',
  today: 'Today',
  yesterday: 'Yesterday',
  emptyTitle: 'No meetings yet',
  emptyDescription: 'Record your first meeting and its transcript and notes will appear here.',
  startRecording: 'Start recording',
  noMatchesTitle: 'No meetings match',
  noMatchesDescription: 'Change the filters, or clear them to see every meeting.',
  clearFilters: 'Clear filters',
  duration: (minutes: number) =>
    minutes < 60
      ? `${String(minutes)}m`
      : `${String(Math.floor(minutes / 60))}h ${String(minutes % 60)}m`,
  moreAttendees: (count: number) => `+${String(count)}`,
  select: (title: string) => `Select ${title}`,
  tabs: {
    label: 'Views',
    all: 'All meetings',
    save: 'Save view',
    saveAs: 'Save as a view',
    name: 'View name',
    namePlaceholder: 'Customer calls',
    confirm: 'Save',
    update: 'Update view',
    rename: 'Rename',
    delete: 'Delete view',
    actions: (name: string) => `${name} view options`,
  },
  filter: {
    button: 'Filter',
    search: 'Filter by…',
    nothing: 'No filter matches.',
    remove: (label: string) => `Remove filter: ${label}`,
    groups: {
      date: 'Date',
      people: 'People',
      companies: 'Companies',
      tags: 'Tags',
      folders: 'Folders',
      more: 'More',
    },
    presets: [
      { label: 'Today', days: 0 },
      { label: 'Last 7 days', days: 6 },
      { label: 'Last 30 days', days: 29 },
    ],
    range: (from: string | null, to: string | null) =>
      from === to && from !== null ? from : `${from ?? '…'} – ${to ?? '…'}`,
    hasItems: 'Has action items',
    audience: { internal: 'Internal', external: 'External' },
  },
  bulk: {
    label: 'Selected meetings',
    selected: (count: number) => `${plural(count, 'meeting', 'meetings')} selected`,
    tag: 'Tag',
    file: 'Add to folder',
    export: 'Export',
    exportAs: 'Export as',
    exported: (count: number, folder: string) =>
      `Exported ${plural(count, 'meeting', 'meetings')} to ${folder}`,
    merge: 'Merge',
    automation: 'Run automation',
    wip: 'WIP',
    delete: 'Delete',
    clear: 'Clear selection',
    tagged: (count: number) => `Tagged ${plural(count, 'meeting', 'meetings')}`,
    filed: (count: number) => `Added ${plural(count, 'meeting', 'meetings')} to the folder`,
    failed: 'That did not work. Try again.',
  },
  trash: {
    title: 'Trash',
    nav: 'Trash',
    description: 'Meetings stay here for 30 days, then they and their recordings are deleted.',
    emptyTitle: 'The trash is empty',
    emptyDescription: 'Deleted meetings wait here for 30 days before they are gone for good.',
    moved: (count: number) => `Moved ${plural(count, 'meeting', 'meetings')} to the trash`,
    undo: 'Undo',
    restore: 'Restore',
    restored: (count: number) => `Restored ${plural(count, 'meeting', 'meetings')}`,
    deleteForever: 'Delete forever',
    confirmTitle: (count: number) => `Delete ${plural(count, 'meeting', 'meetings')} forever?`,
    confirmDescription:
      'Their transcripts, notes and recordings will be deleted. This cannot be undone.',
    cancel: 'Cancel',
    deleted: (count: number) => `Deleted ${plural(count, 'meeting', 'meetings')} forever`,
    purgesIn: (days: number) =>
      days <= 0 ? 'Deleted today' : `Deleted in ${plural(days, 'day', 'days')}`,
  },
  merge: {
    title: (count: number) => `Merge ${plural(count, 'meeting', 'meetings')}?`,
    description:
      'Their transcripts are joined in time order into the earliest meeting, with everyone who attended, their tags, folders, action items and notes. Each recording stays playable.',
    regenerate: 'The AI notes will offer to regenerate once you open the merged meeting.',
    confirm: 'Merge',
    cancel: 'Cancel',
    done: 'Meetings merged',
    open: 'Open',
    needsTwo: 'Select at least two meetings to merge.',
  },
} as const;
