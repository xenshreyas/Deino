import { describe, expect, it } from 'vitest';

import { FIXTURE_CALENDAR, fixtureCalendarEvents } from './calendar-fixtures';
import { commands } from './calendar';
import { fixtureUuid } from './fixture-ids';
import { loadFixtureMeetings } from './fixtures';

const DAY_MS = 24 * 60 * 60 * 1000;

/** The next two weeks from now. */
function twoWeeks() {
  const now = Date.now();
  return {
    start: new Date(now - DAY_MS).toISOString(),
    end: new Date(now + 14 * DAY_MS).toISOString(),
  };
}

describe('calendar fixtures', () => {
  it('serve each recorded event under the ID the Rust seeder gives it', () => {
    const events = fixtureCalendarEvents(loadFixtureMeetings(), new Date());

    const standup = events.find(
      (event) => event.id === fixtureUuid('event/meeting-fixture-standup'),
    );

    expect(standup?.calendarId).toBe(FIXTURE_CALENDAR.id);
    expect(standup?.seriesId).toBe(standup?.icalUid);
  });
});

describe('calendar mock', () => {
  it('lists the coming copies by start, and none once the calendar is hidden', async () => {
    const shown = await commands.upcoming(twoWeeks());
    const starts = shown.status === 'ok' ? shown.data.map((event) => event.startAt) : [];

    expect(starts.length).toBeGreaterThan(0);
    expect([...starts].sort()).toEqual(starts);

    await commands.setCalendarVisible(FIXTURE_CALENDAR.id, false);
    expect(await commands.upcoming(twoWeeks())).toEqual({ status: 'ok', data: [] });
    await commands.setCalendarVisible(FIXTURE_CALENDAR.id, true);
  });

  it('rejects a range that ends before it starts, like the Rust service', async () => {
    const range = twoWeeks();

    const backwards = await commands.upcoming({ start: range.end, end: range.start });

    expect(backwards).toMatchObject({ status: 'error', error: { code: 'invalid' } });
  });
});
