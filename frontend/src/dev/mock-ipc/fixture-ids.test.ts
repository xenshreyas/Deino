import { describe, expect, it } from 'vitest';

import { fixtureUuid } from './fixture-ids';

describe('fixtureUuid', () => {
  it('matches the Rust seeder for the same key', () => {
    // The same value is asserted in crates/deino-testkit/src/fixture/ids.rs.
    expect(fixtureUuid('segment/meeting-fixture-standup/0')).toBe(
      'c7874d53-3bad-8a77-b822-b86e675a1cbc',
    );
  });

  it('gives different keys different version 8 ids', () => {
    const a = fixtureUuid('person/a@deino.dev');
    const b = fixtureUuid('person/b@deino.dev');

    expect(a).not.toBe(b);
    expect(a).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });
});
