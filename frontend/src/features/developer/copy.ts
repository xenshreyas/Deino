/** Every string the developer tools show, in one place (§3A). */
export const copy = {
  title: 'Sample meetings',
  description:
    'Loads eight realistic sample meetings with transcripts into this database. Loading them again restores them; your own meetings are left alone.',
  seed: 'Load sample meetings',
  seeding: 'Loading…',
  seeded: (meetings: number, segments: number) =>
    `Loaded ${meetings} sample meetings with ${segments.toLocaleString('en-US')} transcript segments.`,
} as const;
